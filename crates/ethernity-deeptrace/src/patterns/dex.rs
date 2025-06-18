use super::PatternDetector;
use crate::{analyzer::TraceAnalysisResult, DetectedPattern, PatternType, TokenTransfer};
use async_trait::async_trait;
use ethereum_types::Address;
use std::collections::HashMap;

pub struct DexPatternDetector;

impl DexPatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for DexPatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::TokenSwap
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        if analysis.token_transfers.len() >= 2 {
            let mut token_groups: HashMap<Address, Vec<&TokenTransfer>> = HashMap::new();

            for transfer in &analysis.token_transfers {
                token_groups.entry(transfer.token_address).or_default().push(transfer);
            }

            if token_groups.len() >= 2 {
                let mut confidence = 0.6;
                let mut addresses = Vec::new();
                let mut data = serde_json::Map::new();

                let tokens: Vec<_> = token_groups.keys().collect();
                for (i, &token) in tokens.iter().enumerate() {
                    addresses.push(*token);
                    data.insert(format!("token_{}", i), serde_json::Value::String(format!("{:?}", token)));
                }

                let mut has_bidirectional = false;
                for transfers in token_groups.values() {
                    if transfers.len() > 1 {
                        has_bidirectional = true;
                        break;
                    }
                }

                if has_bidirectional {
                    confidence += 0.2;
                }

                if confidence >= self.min_confidence() {
                    let pattern = DetectedPattern {
                        pattern_type: PatternType::TokenSwap,
                        confidence,
                        addresses,
                        data: serde_json::Value::Object(data),
                        description: "Padrão de swap de tokens detectado".to_string(),
                    };

                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }
}
