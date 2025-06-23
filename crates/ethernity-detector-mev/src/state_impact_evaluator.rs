use crate::tx_aggregator::{AnnotatedTx, TxGroup};
use ethernity_core::types::TransactionHash;
use ethereum_types::{Address, H256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

pub struct StateImpactEvaluator;

impl StateImpactEvaluator {
    pub fn evaluate(group: &TxGroup, victims: &[VictimInput], snapshot: &StateSnapshot) -> GroupImpact {
        let pool_type = Self::resolve_pool_type(group);
        let mut impacts = Vec::new();
        let mut expected_profit = 0.0;
        let mut prev_slippage: Option<f64> = None;
        let mut convexity_integrity_score = 1.0;

        for v in victims {
            let expected = match pool_type {
                PoolType::V2 | PoolType::Unknown => Self::expected_out_v2(v.amount_in, snapshot.reserve_in, snapshot.reserve_out),
                PoolType::V3 => {
                    let sp = snapshot.sqrt_price_x96.unwrap_or(0.0);
                    Self::expected_out_v3(v.amount_in, sp)
                },
                PoolType::Lending => Self::expected_out_v2(v.amount_in, snapshot.reserve_in, snapshot.reserve_out),
            };
            let slippage_tolerated = if expected > 0.0 { ((expected - v.amount_out_min) / expected) * 100.0 } else { 0.0 };
            let slippage_baseline = 3.0;
            let slippage_adjusted = (slippage_tolerated + slippage_baseline) / 2.0;
            if let Some(prev) = prev_slippage {
                let delta = slippage_tolerated - prev;
                if delta.abs() > 5.0 {
                    convexity_integrity_score *= 0.4;
                }
            }
            prev_slippage = Some(slippage_tolerated);
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
        if convexity_integrity_score < 0.5 { opportunity_score *= 0.5; }

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

    fn resolve_pool_type(group: &TxGroup) -> PoolType {
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
}

