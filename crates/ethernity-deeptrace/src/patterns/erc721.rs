use super::PatternDetector;
use crate::{analyzer::TraceAnalysisResult, DetectedPattern, PatternType, ContractType};
use async_trait::async_trait;

pub struct Erc721PatternDetector;

impl Erc721PatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for Erc721PatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::Erc721Creation
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        for creation in &analysis.contract_creations {
            if matches!(creation.contract_type, ContractType::Erc721Token) {
                let mut data = serde_json::Map::new();
                data.insert("contract_address".to_string(), serde_json::Value::String(format!("{:?}", creation.contract_address)));
                data.insert("creator".to_string(), serde_json::Value::String(format!("{:?}", creation.creator)));

                let pattern = DetectedPattern {
                    pattern_type: PatternType::Erc721Creation,
                    confidence: 0.9,
                    addresses: vec![creation.contract_address, creation.creator],
                    data: serde_json::Value::Object(data),
                    description: "Criação de token ERC721 (NFT) detectada".to_string(),
                };

                patterns.push(pattern);
            }
        }

        Ok(patterns)
    }
}
