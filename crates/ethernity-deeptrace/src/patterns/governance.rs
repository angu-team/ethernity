use super::PatternDetector;
use crate::{analyzer::TraceAnalysisResult, DetectedPattern, PatternType};
use async_trait::async_trait;
use ethereum_types::Address;

pub struct GovernancePatternDetector;

impl GovernancePatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for GovernancePatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::Governance
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        let governance_signatures = [
            &[0xda, 0x35, 0xc6, 0x64],
            &[0x15, 0x37, 0x3e, 0x3d],
            &[0xfe, 0x0d, 0x94, 0xc1],
            &[0x40, 0xe5, 0x8e, 0xe5],
        ];

        analysis.call_tree.traverse_preorder(|node| {
            if !node.input.is_empty() && node.input.len() >= 4 {
                let function_sig = &node.input[0..4];

                for &gov_sig in &governance_signatures {
                    if function_sig == gov_sig {
                        let mut data = serde_json::Map::new();
                        data.insert("contract".to_string(), serde_json::Value::String(format!("{:?}", node.to.unwrap_or_else(|| Address::zero()))));
                        data.insert("caller".to_string(), serde_json::Value::String(format!("{:?}", node.from)));
                        data.insert("function_signature".to_string(), serde_json::Value::String(hex::encode(function_sig)));

                        let pattern = DetectedPattern {
                            pattern_type: PatternType::Governance,
                            confidence: 0.85,
                            addresses: vec![node.from, node.to.unwrap_or_else(|| Address::zero())],
                            data: serde_json::Value::Object(data),
                            description: "Atividade de governança detectada".to_string(),
                        };

                        patterns.push(pattern);
                        break;
                    }
                }
            }
        });

        Ok(patterns)
    }
}
