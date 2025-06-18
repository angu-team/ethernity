use super::PatternDetector;
use crate::{analyzer::TraceAnalysisResult, DetectedPattern, PatternType};
use async_trait::async_trait;
use ethereum_types::U256;

pub struct FlashLoanPatternDetector;

impl FlashLoanPatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for FlashLoanPatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::FlashLoan
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        if analysis.token_transfers.len() >= 3 {
            let first_transfer = &analysis.token_transfers[0];
            let last_transfer = analysis.token_transfers.last().unwrap();

            if first_transfer.token_address == last_transfer.token_address {
                if first_transfer.to == last_transfer.from &&
                    first_transfer.from == last_transfer.to {

                    if last_transfer.amount >= first_transfer.amount {
                        let fee_ratio = if first_transfer.amount > U256::zero() {
                            (last_transfer.amount - first_transfer.amount).as_u128() as f64 /
                                first_transfer.amount.as_u128() as f64
                        } else {
                            0.0
                        };

                        if fee_ratio >= 0.0005 && fee_ratio <= 0.01 {
                            let mut data = serde_json::Map::new();
                            data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", first_transfer.token_address)));
                            data.insert("amount".to_string(), serde_json::Value::String(first_transfer.amount.to_string()));
                            data.insert("fee_ratio".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(fee_ratio).unwrap()));
                            data.insert("intermediate_operations".to_string(), serde_json::Value::Number(serde_json::Number::from(analysis.token_transfers.len() - 2)));

                            let pattern = DetectedPattern {
                                pattern_type: PatternType::FlashLoan,
                                confidence: 0.85,
                                addresses: vec![first_transfer.token_address, first_transfer.from, first_transfer.to],
                                data: serde_json::Value::Object(data),
                                description: "Flash loan detectado".to_string(),
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
