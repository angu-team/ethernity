use super::PatternDetector;
use crate::{analyzer::TraceAnalysisResult, DetectedPattern, PatternType};
use async_trait::async_trait;
use ethereum_types::U256;

pub struct LendingPatternDetector;

impl LendingPatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for LendingPatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::Liquidity
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        for window in analysis.token_transfers.windows(2) {
            if let [transfer1, transfer2] = window {
                if transfer1.token_address == transfer2.token_address &&
                    transfer1.from == transfer2.to &&
                    transfer1.to == transfer2.from {

                    let ratio = if transfer2.amount > U256::zero() {
                        transfer1.amount.as_u128() as f64 / transfer2.amount.as_u128() as f64
                    } else {
                        0.0
                    };

                    if ratio > 1.1 && ratio < 100.0 {
                        let mut data = serde_json::Map::new();
                        data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", transfer1.token_address)));
                        data.insert("principal".to_string(), serde_json::Value::String(transfer1.amount.to_string()));
                        data.insert("repayment".to_string(), serde_json::Value::String(transfer2.amount.to_string()));
                        data.insert("interest_ratio".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(ratio - 1.0).unwrap()));

                        let pattern = DetectedPattern {
                            pattern_type: PatternType::Liquidity,
                            confidence: 0.75,
                            addresses: vec![transfer1.token_address, transfer1.from, transfer1.to],
                            data: serde_json::Value::Object(data),
                            description: "Padrão de empréstimo/liquidez detectado".to_string(),
                        };

                        patterns.push(pattern);
                    }
                }
            }
        }

        Ok(patterns)
    }
}
