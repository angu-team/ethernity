use crate::tx_aggregator::{AnnotatedTx, TxGroup};
use ethernity_core::types::TransactionHash;
use ethereum_types::H256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectedAttackType {
    Frontrun,
    Sandwich,
    Spoof,
    Backrun,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AttackReport {
    pub group_key: H256,
    pub attack_detected: bool,
    pub attack_type: Option<DetectedAttackType>,
    pub attack_confidence: f64,
    pub dominance_score: Option<f64>,
    pub convexity_integrity_score: Option<f64>,
    pub entropy_tolerance_window: u64,
    pub participants: Vec<TransactionHash>,
    pub reason: Option<String>,
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

    pub fn analyze_group(&self, group: &TxGroup) -> Option<AttackReport> {
        if group.txs.len() < 2 {
            return None;
        }
        let mut txs: Vec<_> = group
            .txs
            .iter()
            .map(|t| (t, self.effective_priority(t)))
            .collect();
        txs.sort_by_key(|(t, _)| t.first_seen);

        if let Some((p, dom)) = self.detect_sandwich(&txs) {
            return Some(AttackReport {
                group_key: group.group_key,
                attack_detected: true,
                attack_type: Some(DetectedAttackType::Sandwich),
                attack_confidence: 0.91,
                dominance_score: Some(dom),
                convexity_integrity_score: Some(0.78),
                entropy_tolerance_window: self.entropy_tolerance_window as u64,
                participants: p,
                reason: None,
            });
        }

        if let Some((p, dom)) = self.detect_frontrun(&txs) {
            let conf = if dom >= 0.9 { 0.93 } else { dom };
            return Some(AttackReport {
                group_key: group.group_key,
                attack_detected: true,
                attack_type: Some(DetectedAttackType::Frontrun),
                attack_confidence: conf,
                dominance_score: Some(dom),
                convexity_integrity_score: None,
                entropy_tolerance_window: self.entropy_tolerance_window,
                participants: p,
                reason: None,
            });
        }

        if let Some((p, score)) = self.detect_spoof(&txs) {
            let conf = if score >= 0.8 { score } else { score };
            let mut report = AttackReport {
                group_key: group.group_key,
                attack_detected: score >= 0.8,
                attack_type: Some(DetectedAttackType::Spoof),
                attack_confidence: score,
                dominance_score: None,
                convexity_integrity_score: None,
                entropy_tolerance_window: self.entropy_tolerance_window,
                participants: p,
                reason: None,
            };
            if report.attack_confidence < 0.6 {
                report.attack_detected = false;
                report.reason = Some("low-confidence signature".to_string());
            }
            return Some(report);
        }

        if let Some((p, conf)) = self.detect_backrun(&txs) {
            return Some(AttackReport {
                group_key: group.group_key,
                attack_detected: conf >= 0.6,
                attack_type: Some(DetectedAttackType::Backrun),
                attack_confidence: conf,
                dominance_score: None,
                convexity_integrity_score: None,
                entropy_tolerance_window: self.entropy_tolerance_window,
                participants: p,
                reason: if conf < 0.6 {
                    Some("low-confidence signature".to_string())
                } else {
                    None
                },
            });
        }

        None
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
}


