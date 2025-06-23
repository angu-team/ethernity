use crate::tx_aggregator::TxGroup;
use crate::traits::ImpactModel;
use ethernity_core::types::TransactionHash;
use ethereum_types::{Address, H256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PoolType {
    V2,
    V3,
    Lending,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub reserve_in: f64,
    pub reserve_out: f64,
    pub sqrt_price_x96: Option<f64>,
    pub liquidity: Option<f64>,
    pub state_lag_blocks: u64,
    pub reorg_risk_level: String,
    pub volatility_flag: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VictimInput {
    pub tx_hash: TransactionHash,
    pub amount_in: f64,
    pub amount_out_min: f64,
    pub token_behavior_unknown: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VictimImpact {
    pub tx_hash: TransactionHash,
    pub amount_in: f64,
    pub expected_amount_out: f64,
    pub amount_out_min: f64,
    pub slippage_tolerated: f64,
    pub slippage_baseline: f64,
    pub slippage_adjusted: f64,
    pub token_behavior_unknown: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupImpact {
    pub group_id: H256,
    pub tokens: Vec<Address>,
    pub victims: Vec<VictimImpact>,
    pub opportunity_score: f64,
    pub expected_profit_backrun: f64,
    pub state_confidence: f64,
    pub impact_certainty: f64,
    pub execution_assumption: String,
    pub reorg_risk_level: String,
}

use std::sync::Arc;
use parking_lot::Mutex;

pub trait CurveModel: Send + Sync {
    fn expected_out(&self, amount_in: f64, snapshot: &StateSnapshot) -> f64;
    fn apply_trade(&self, amount_in: f64, snapshot: &mut StateSnapshot) {
        let out = self.expected_out(amount_in, snapshot);
        snapshot.reserve_in += amount_in;
        snapshot.reserve_out -= out;
    }
}

#[derive(Debug, Default, Clone)]
pub struct ConstantProductCurve;

impl CurveModel for ConstantProductCurve {
    fn expected_out(&self, amount_in: f64, snapshot: &StateSnapshot) -> f64 {
        StateImpactEvaluator::expected_out_v2(amount_in, snapshot.reserve_in, snapshot.reserve_out)
    }

    fn apply_trade(&self, amount_in: f64, snapshot: &mut StateSnapshot) {
        let out = self.expected_out(amount_in, snapshot);
        snapshot.reserve_in += amount_in;
        snapshot.reserve_out -= out;
    }
}

#[derive(Debug, Default, Clone)]
pub struct UniswapV3Curve;

impl CurveModel for UniswapV3Curve {
    fn expected_out(&self, amount_in: f64, snapshot: &StateSnapshot) -> f64 {
        let sp = snapshot.sqrt_price_x96.unwrap_or(0.0);
        StateImpactEvaluator::expected_out_v3(amount_in, sp)
    }
}

#[derive(Clone)]
pub struct ImpactModelParams {
    pub liquidity: f64,
    pub slippage_curve: f64,
    pub convexity: f64,
    pub curve_model: Arc<dyn CurveModel>,
    pub lightweight_simulation: bool,
}

impl Default for ImpactModelParams {
    fn default() -> Self {
        Self {
            liquidity: 1.0,
            slippage_curve: 3.0,
            convexity: 0.5,
            curve_model: Arc::new(ConstantProductCurve),
            lightweight_simulation: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SlippageHistory {
    window: usize,
    values: Vec<f64>,
}

impl SlippageHistory {
    pub fn new(window: usize) -> Self {
        Self { window, values: Vec::new() }
    }

    pub fn record(&mut self, v: f64) {
        self.values.push(v);
        if self.values.len() > self.window {
            self.values.remove(0);
        }
    }

    pub fn average(&self) -> f64 {
        if self.values.is_empty() { 0.0 } else { self.values.iter().sum::<f64>() / self.values.len() as f64 }
    }

    pub fn is_empty(&self) -> bool { self.values.is_empty() }
}

pub struct StateImpactEvaluator {
    params: ImpactModelParams,
    slippage_history: Mutex<SlippageHistory>,
}

impl Default for StateImpactEvaluator {
    fn default() -> Self {
        Self { params: ImpactModelParams::default(), slippage_history: Mutex::new(SlippageHistory::new(10)) }
    }
}

impl StateImpactEvaluator {
    pub fn new(params: ImpactModelParams) -> Self {
        Self { params, slippage_history: Mutex::new(SlippageHistory::new(10)) }
    }

    pub fn evaluate(group: &TxGroup, victims: &[VictimInput], snapshot: &StateSnapshot) -> GroupImpact {
        Self::default().evaluate_inner(group, victims, snapshot)
    }

    fn evaluate_inner(&self, group: &TxGroup, victims: &[VictimInput], snapshot: &StateSnapshot) -> GroupImpact {
        let pool_type = Self::resolve_pool_type(group);
        let mut impacts = Vec::new();
        let mut expected_profit = 0.0;
        let mut prev_slippage: Option<f64> = None;
        let mut convexity_integrity_score = 1.0;
        let mut snapshot_local = snapshot.clone();

        for v in victims {
            let expected = match pool_type {
                PoolType::V2 | PoolType::Unknown => self.params.curve_model.expected_out(v.amount_in, &snapshot_local),
                PoolType::V3 => self.params.curve_model.expected_out(v.amount_in, &snapshot_local),
                PoolType::Lending => self.params.curve_model.expected_out(v.amount_in, &snapshot_local),
            };
            let slippage_tolerated = if expected > 0.0 { ((expected - v.amount_out_min) / expected) * 100.0 } else { 0.0 };
            let baseline_dynamic = {
                let hist = self.slippage_history.lock();
                if hist.is_empty() { self.params.slippage_curve } else { hist.average() }
            };
            let slippage_baseline = baseline_dynamic;
            let slippage_adjusted = (slippage_tolerated + slippage_baseline) / 2.0;
            if let Some(prev) = prev_slippage {
                let delta = slippage_tolerated - prev;
                if delta.abs() > 5.0 {
                    convexity_integrity_score *= 0.4;
                }
            }
            prev_slippage = Some(slippage_tolerated);
            self.slippage_history.lock().record(slippage_tolerated);
            expected_profit += expected - v.amount_out_min;
            impacts.push(VictimImpact {
                tx_hash: v.tx_hash,
                amount_in: v.amount_in,
                expected_amount_out: expected,
                amount_out_min: v.amount_out_min,
                slippage_tolerated,
                slippage_baseline,
                slippage_adjusted,
                token_behavior_unknown: v.token_behavior_unknown,
            });
            if self.params.lightweight_simulation {
                self.params.curve_model.apply_trade(v.amount_in, &mut snapshot_local);
            }
        }

        let mut state_confidence = 1.0;
        if snapshot.state_lag_blocks > 2 { state_confidence -= 0.2; }
        match snapshot.reorg_risk_level.as_str() {
            "high" => state_confidence -= 0.3,
            "medium" => state_confidence -= 0.1,
            _ => {},
        }
        if state_confidence < 0.0 { state_confidence = 0.0; }
        let impact_certainty = if impacts.iter().any(|v| v.token_behavior_unknown) { 0.61 } else { 0.9 };
        let mut opportunity_score = (state_confidence + impact_certainty) / 2.0;
        if convexity_integrity_score < self.params.convexity { opportunity_score *= 0.5; }

        GroupImpact {
            group_id: group.group_key,
            tokens: group.token_paths.clone(),
            victims: impacts,
            opportunity_score,
            expected_profit_backrun: expected_profit,
            state_confidence,
            impact_certainty,
            execution_assumption: "ideal".to_string(),
            reorg_risk_level: snapshot.reorg_risk_level.clone(),
        }
    }

    pub fn resolve_pool_type(group: &TxGroup) -> PoolType {
        let tags: Vec<String> = group.txs.iter().flat_map(|t| t.tags.clone()).collect();
        if tags.iter().any(|t| t == "swap-v2") { PoolType::V2 }
        else if tags.iter().any(|t| t == "swap-v3") { PoolType::V3 }
        else if tags.iter().any(|t| t == "lending") { PoolType::Lending }
        else { PoolType::Unknown }
    }

    fn expected_out_v2(amount_in: f64, reserve_in: f64, reserve_out: f64) -> f64 {
        (amount_in * 997.0 * reserve_out) / (reserve_in * 1000.0 + amount_in * 997.0)
    }

    fn expected_out_v3(amount_in: f64, sqrt_price_x96: f64) -> f64 {
        let ratio = (sqrt_price_x96 * sqrt_price_x96) / 2_f64.powi(192);
        amount_in * ratio
    }

    /// Evaluates groups from [`SnapshotEvent`] and emits [`ImpactEvent`].
    pub async fn process_stream(
        mut rx: tokio::sync::mpsc::Receiver<crate::events::SnapshotEvent>,
        tx: tokio::sync::mpsc::Sender<crate::events::ImpactEvent>,
    ) {
        while let Some(ev) = rx.recv().await {
            if let Some(snapshot) = ev.snapshots.values().next() {
                let impact = Self::evaluate(&ev.group, &[], snapshot);
                let _ = tx
                    .send(crate::events::ImpactEvent {
                        group: ev.group.clone(),
                        impact,
                    })
                    .await;
            }
        }
    }
}

impl ImpactModel for StateImpactEvaluator {
    fn evaluate_group(
        &self,
        group: &TxGroup,
        victims: &[VictimInput],
        snapshot: &StateSnapshot,
    ) -> GroupImpact {
        self.evaluate_inner(group, victims, snapshot)
    }
}

