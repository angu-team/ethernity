use super::PatternDetector;
use crate::{analyzer::TraceAnalysisResult, DetectedPattern, PatternType, ContractType};
use async_trait::async_trait;

pub struct Erc20PatternDetector;

impl Erc20PatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for Erc20PatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::Erc20Creation
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        for creation in &analysis.contract_creations {
            if matches!(creation.contract_type, ContractType::Erc20Token) {
                let mut data = serde_json::Map::new();
                data.insert("contract_address".to_string(), serde_json::Value::String(format!("{:?}", creation.contract_address)));
                data.insert("creator".to_string(), serde_json::Value::String(format!("{:?}", creation.creator)));

                let pattern = DetectedPattern {
                    pattern_type: PatternType::Erc20Creation,
                    confidence: 0.9,
                    addresses: vec![creation.contract_address, creation.creator],
                    data: serde_json::Value::Object(data),
                    description: "Criação de token ERC20 detectada".to_string(),
                };

                patterns.push(pattern);
            }
        }

        Ok(patterns)
    }
}
