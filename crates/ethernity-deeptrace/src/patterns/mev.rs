use super::PatternDetector;
use crate::{analyzer::TraceAnalysisResult, DetectedPattern, PatternType};
use async_trait::async_trait;
use ethereum_types::{Address, U256};
use std::collections::HashMap;

pub struct MevPatternDetector;

impl MevPatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for MevPatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::Arbitrage
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        if analysis.token_transfers.len() >= 4 {
            let mut token_flows: HashMap<Address, Vec<(Address, Address, U256)>> = HashMap::new();

            for transfer in &analysis.token_transfers {
                token_flows.entry(transfer.token_address)
                    .or_default()
                    .push((transfer.from, transfer.to, transfer.amount));
            }

            for (token, flows) in token_flows {
                if flows.len() >= 2 {
                    let mut net_flow: HashMap<Address, i128> = HashMap::new();

                    for (from, to, amount) in flows {
                        let amount_i128 = amount.as_u128() as i128;
                        *net_flow.entry(from).or_default() -= amount_i128;
                        *net_flow.entry(to).or_default() += amount_i128;
                    }

                    for (address, net) in net_flow {
                        if net > 0 && net as u128 > 1000 {
                            let mut data = serde_json::Map::new();
                            data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", token)));
                            data.insert("arbitrageur".to_string(), serde_json::Value::String(format!("{:?}", address)));
                            data.insert("profit".to_string(), serde_json::Value::String(net.to_string()));

                            let pattern = DetectedPattern {
                                pattern_type: PatternType::Arbitrage,
                                confidence: 0.8,
                                addresses: vec![token, address],
                                data: serde_json::Value::Object(data),
                                description: "Padrão de arbitragem MEV detectado".to_string(),
                            };

                            patterns.push(pattern);
                        }
                    }
                }
            }
        }

        Ok(patterns)
    }
}
