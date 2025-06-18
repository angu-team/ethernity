use super::PatternDetector;
use crate::{analyzer::TraceAnalysisResult, DetectedPattern, PatternType, ContractType};
use async_trait::async_trait;
use ethereum_types::U256;

pub struct RugPullPatternDetector;

impl RugPullPatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for RugPullPatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::RugPull
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        for creation in &analysis.contract_creations {
            if matches!(creation.contract_type, ContractType::Erc20Token) {
                let mut suspicious_transfers = Vec::new();
                let mut total_to_creator = U256::zero();

                for transfer in &analysis.token_transfers {
                    if transfer.token_address == creation.contract_address &&
                        transfer.to == creation.creator {
                        suspicious_transfers.push(transfer);
                        total_to_creator += transfer.amount;
                    }
                }

                if !suspicious_transfers.is_empty() && total_to_creator > U256::from(1000000) {
                    let mut data = serde_json::Map::new();
                    data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", creation.contract_address)));
                    data.insert("creator".to_string(), serde_json::Value::String(format!("{:?}", creation.creator)));
                    data.insert("suspicious_amount".to_string(), serde_json::Value::String(total_to_creator.to_string()));
                    data.insert("transfer_count".to_string(), serde_json::Value::Number(serde_json::Number::from(suspicious_transfers.len())));

                    let confidence = if suspicious_transfers.len() > 3 { 0.9 } else { 0.7 };

                    let pattern = DetectedPattern {
                        pattern_type: PatternType::RugPull,
                        confidence,
                        addresses: vec![creation.contract_address, creation.creator],
                        data: serde_json::Value::Object(data),
                        description: "Possível rug pull detectado".to_string(),
                    };

                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }
}
