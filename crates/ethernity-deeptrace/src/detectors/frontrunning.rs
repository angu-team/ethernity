use super::{DetectedEvent, EventSeverity, SpecializedDetector};
use crate::analyzer::TraceAnalysisResult;
use async_trait::async_trait;
use ethereum_types::Address;

/// Detector de frontrunning
pub struct FrontrunningDetector;

impl FrontrunningDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SpecializedDetector for FrontrunningDetector {
    fn name(&self) -> &str {
        "FrontrunningDetector"
    }

    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut events = Vec::new();

        let mut call_nodes = Vec::new();
        analysis.call_tree.traverse_preorder(|node| {
            call_nodes.push(node.clone());
        });

        for window in call_nodes.windows(2) {
            if let [call1, call2] = window {
                if call1.to == call2.to && call1.to.is_some() {
                    if call1.input.len() >= 4 && call2.input.len() >= 4 {
                        if call1.input[0..4] == call2.input[0..4] {
                            if call1.from != call2.from {
                                let mut data = serde_json::Map::new();
                                data.insert("contract".to_string(), serde_json::Value::String(format!("{:?}", call1.to)));
                                data.insert("frontrunner".to_string(), serde_json::Value::String(format!("{:?}", call1.from)));
                                data.insert("victim".to_string(), serde_json::Value::String(format!("{:?}", call2.from)));
                                data.insert("function".to_string(), serde_json::Value::String(hex::encode(&call1.input[0..4])));

                                let event = DetectedEvent {
                                    event_type: "frontrunning".to_string(),
                                    confidence: 0.75,
                                    addresses: vec![call1.to.unwrap_or_else(|| Address::zero()), call1.from, call2.from],
                                    data: serde_json::Value::Object(data),
                                    description: "Possível frontrunning detectado".to_string(),
                                    severity: EventSeverity::Medium,
                                };

                                events.push(event);
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }
}
