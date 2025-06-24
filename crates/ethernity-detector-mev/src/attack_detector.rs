use crate::tx_aggregator::{AnnotatedTx, TxGroup};
use ethernity_core::types::TransactionHash;
use ethereum_types::H256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttackType {
    Frontrun { justification: String },
    Sandwich { justification: String },
    Spoof { justification: String },
    Backrun { justification: String },
    CrossChain { justification: String },
    FlashLoan { justification: String },
    MultiToken { justification: String },
    Layer2 { justification: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackVerdict {
    pub group_key: H256,
    pub attack_types: Vec<AttackType>,
    pub confidence: f64,
    pub reconsiderable: bool,
}


pub struct AttackDetector {
    base_fee: f64,
    entropy_tolerance_window: u64,
}

impl AttackDetector {
    pub fn new(base_fee: f64, entropy_tolerance_window: u64) -> Self {
        Self { base_fee, entropy_tolerance_window }
    }

    fn effective_priority(&self, tx: &AnnotatedTx) -> f64 {
        let max_priority = tx.max_priority_fee_per_gas.unwrap_or(tx.gas_price);
        let diff = tx.gas_price - self.base_fee;
        let eff = if diff < 0.0 { 0.0 } else { diff };
        max_priority.min(eff)
    }

    pub fn analyze_group(&self, group: &TxGroup) -> Option<AttackVerdict> {
        if group.txs.len() < 2 {
            return None;
        }
        let mut txs: Vec<_> = group
            .txs
            .iter()
            .map(|t| (t, self.effective_priority(t)))
            .collect();
        txs.sort_by_key(|(t, _)| t.first_seen);

        let mut attacks = Vec::new();
        let mut confidences = Vec::new();

        if let Some((_, dom)) = self.detect_sandwich(&txs) {
            attacks.push(AttackType::Sandwich { justification: "sandwich pattern".to_string() });
            confidences.push(0.91);
        }

        if let Some((_p, dom)) = self.detect_frontrun(&txs) {
            let conf = if dom >= 0.9 { 0.93 } else { dom };
            attacks.push(AttackType::Frontrun { justification: format!("priority dominance {:.2}", dom) });
            confidences.push(conf);
        }

        if let Some((_p, score)) = self.detect_spoof(&txs) {
            attacks.push(AttackType::Spoof { justification: "suspicious gas pattern".to_string() });
            confidences.push(score);
        }

        if let Some((_p, conf)) = self.detect_backrun(&txs) {
            attacks.push(AttackType::Backrun { justification: "backrun sequence".to_string() });
            confidences.push(conf);
        }

        if let Some((_p, conf)) = self.detect_cross_chain(group) {
            attacks.push(AttackType::CrossChain { justification: "cross-chain interaction".to_string() });
            confidences.push(conf);
        }

        if let Some((_p, conf)) = self.detect_flash_loan(group) {
            attacks.push(AttackType::FlashLoan { justification: "flash loan activity".to_string() });
            confidences.push(conf);
        }

        if let Some((_p, conf)) = self.detect_multi_token(group) {
            attacks.push(AttackType::MultiToken { justification: "multiple tokens".to_string() });
            confidences.push(conf);
        }

        if let Some((_p, conf)) = self.detect_layer2(group) {
            attacks.push(AttackType::Layer2 { justification: "layer2 mev".to_string() });
            confidences.push(conf);
        }

        if attacks.is_empty() {
            return None;
        }

        let base_conf: f64 = confidences.iter().copied().sum::<f64>() / confidences.len() as f64;
        let complexity_bonus = 0.05 * (attacks.len() as f64 - 1.0);
        let confidence = (base_conf + complexity_bonus).min(0.99);
        let reconsiderable = confidence < 0.8;

        Some(AttackVerdict {
            group_key: group.group_key,
            attack_types: attacks,
            confidence,
            reconsiderable,
        })
    }

    fn detect_frontrun(&self, txs: &[( &AnnotatedTx, f64)]) -> Option<(Vec<TransactionHash>, f64)> {
        for i in 0..txs.len() {
            for j in (i + 1)..txs.len() {
                let dt = txs[j].0.first_seen.saturating_sub(txs[i].0.first_seen);
                if dt > self.entropy_tolerance_window {
                    continue;
                }
                if txs[i].1 > txs[j].1 {
                    let dom = txs[i].1 / (txs[i].1 + txs[j].1);
                    if dom > 0.65 {
                        return Some((vec![txs[i].0.tx_hash, txs[j].0.tx_hash], dom));
                    }
                }
            }
        }
        None
    }

    fn detect_sandwich(&self, txs: &[(&AnnotatedTx, f64)]) -> Option<(Vec<TransactionHash>, f64)> {
        if txs.len() < 3 {
            return None;
        }
        for i in 0..txs.len() - 2 {
            let a = &txs[i];
            for j in (i + 1)..txs.len() - 1 {
                let b = &txs[j];
                let dt1 = b.0.first_seen.saturating_sub(a.0.first_seen);
                if dt1 > self.entropy_tolerance_window {
                    continue;
                }
                for k in (j + 1)..txs.len() {
                    let c = &txs[k];
                    let dt2 = c.0.first_seen.saturating_sub(b.0.first_seen);
                    if dt2 > self.entropy_tolerance_window {
                        continue;
                    }
                    if a.1 > b.1 && c.1 > b.1 {
                        let dom = (a.1 + c.1) / (a.1 + b.1 + c.1);
                        if dom > 0.6 {
                            return Some((vec![a.0.tx_hash, b.0.tx_hash, c.0.tx_hash], dom));
                        }
                    }
                }
            }
        }
        None
    }

    fn detect_spoof(&self, txs: &[(&AnnotatedTx, f64)]) -> Option<(Vec<TransactionHash>, f64)> {
        let avg_gas: f64 = txs.iter().map(|(t, _)| t.gas_price).sum::<f64>() / txs.len() as f64;
        for (tx, _) in txs {
            let anomaly = Self::anomaly_score(tx);
            let high_gas = tx.gas_price > avg_gas * 2.0;
            let likelihood = if high_gas { 0.5 } else { 0.0 } + anomaly;
            if likelihood >= 0.5 {
                return Some((vec![tx.tx_hash], likelihood));
            }
        }
        None
    }

    fn anomaly_score(tx: &AnnotatedTx) -> f64 {
        let data_len = tx.tags.len();
        if data_len == 0 { return 0.0; }
        // use tag text as proxy for calldata presence to avoid new fields
        let unusual = tx.tags.iter().filter(|t| t.len() > 20).count() as f64;
        let score = unusual / data_len as f64;
        if score > 1.0 { 1.0 } else { score }
    }

    fn detect_backrun(&self, txs: &[(&AnnotatedTx, f64)]) -> Option<(Vec<TransactionHash>, f64)> {
        if txs.len() < 2 { return None; }
        let avg_priority: f64 = txs.iter().map(|(_, p)| *p).sum::<f64>() / txs.len() as f64;
        if let Some(last) = txs.last() {
            if last.1 > avg_priority && txs.len() > 2 {
                return Some((txs.iter().map(|(t, _)| t.tx_hash).collect(), 0.7));
            }
        }
        None
    }

    fn detect_cross_chain(&self, group: &TxGroup) -> Option<(Vec<TransactionHash>, f64)> {
        let has_tag = group
            .txs
            .iter()
            .any(|t| t.tags.iter().any(|tg| tg.contains("cross-chain") || tg.contains("bridge")));
        if has_tag && group.token_paths.len() > 2 {
            return Some((group.txs.iter().map(|t| t.tx_hash).collect(), 0.8));
        }
        None
    }

    fn detect_flash_loan(&self, group: &TxGroup) -> Option<(Vec<TransactionHash>, f64)> {
        let has_tag = group
            .txs
            .iter()
            .any(|t| t.tags.iter().any(|tg| tg.contains("flash-loan")));
        if has_tag {
            return Some((group.txs.iter().map(|t| t.tx_hash).collect(), 0.7));
        }
        None
    }

    fn detect_multi_token(&self, group: &TxGroup) -> Option<(Vec<TransactionHash>, f64)> {
        if group.token_paths.len() >= 3 {
            return Some((group.txs.iter().map(|t| t.tx_hash).collect(), 0.6));
        }
        None
    }

    fn detect_layer2(&self, group: &TxGroup) -> Option<(Vec<TransactionHash>, f64)> {
        let has_tag = group
            .txs
            .iter()
            .any(|t| t.tags.iter().any(|tg| tg.contains("l2") || tg.contains("optimism") || tg.contains("arbitrum")));
        if has_tag {
            return Some((group.txs.iter().map(|t| t.tx_hash).collect(), 0.75));
        }
        None
    }

    /// Consumes [`ImpactEvent`]s and emits [`ThreatEvent`].
    pub async fn process_stream(
        &self,
        mut rx: tokio::sync::mpsc::Receiver<crate::events::ImpactEvent>,
        tx: tokio::sync::mpsc::Sender<crate::events::ThreatEvent>,
    ) {
        while let Some(ev) = rx.recv().await {
            if let Some(verdict) = self.analyze_group(&ev.group) {
                let _ = tx.send(crate::events::ThreatEvent { verdict }).await;
            }
        }
    }
}


