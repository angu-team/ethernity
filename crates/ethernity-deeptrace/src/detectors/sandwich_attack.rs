use super::{DetectedEvent, EventSeverity, SpecializedDetector};
use crate::analyzer::TraceAnalysisResult;
use async_trait::async_trait;
use ethereum_types::{Address, U256};

/// Detector de sandwich attacks
pub struct SandwichAttackDetector;

impl SandwichAttackDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SpecializedDetector for SandwichAttackDetector {
    fn name(&self) -> &str {
        "SandwichAttackDetector"
    }

    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut events = Vec::new();

        if analysis.token_transfers.len() >= 3 {
            let mut token_groups = std::collections::HashMap::new();
            for (i, transfer) in analysis.token_transfers.iter().enumerate() {
                token_groups.entry(transfer.token_address)
                    .or_insert_with(Vec::new)
                    .push((i, transfer));
            }

            for (token, transfers) in token_groups {
                if transfers.len() >= 3 {
                    for window in transfers.windows(3) {
                        if let [( _i1, t1 ), ( _i2, t2 ), ( _i3, t3 )] = window {
                            if t1.to == t3.from && t1.from == t3.to {
                                if t2.from != t1.from && t2.to != t1.to {
                                    let profit = if t3.amount > t1.amount {
                                        t3.amount - t1.amount
                                    } else {
                                        U256::zero()
                                    };

                                    if profit > U256::zero() {
                                        let mut data = serde_json::Map::new();
                                        data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", token)));
                                        data.insert("attacker".to_string(), serde_json::Value::String(format!("{:?}", t1.from)));
                                        data.insert("victim".to_string(), serde_json::Value::String(format!("{:?}", t2.from)));
                                        data.insert("profit".to_string(), serde_json::Value::String(profit.to_string()));

                                        let event = DetectedEvent {
                                            event_type: "sandwich_attack".to_string(),
                                            confidence: 0.85,
                                            addresses: vec![token, t1.from, t2.from],
                                            data: serde_json::Value::Object(data),
                                            description: "Possível sandwich attack detectado".to_string(),
                                            severity: EventSeverity::High,
                                        };

                                        events.push(event);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }
}
