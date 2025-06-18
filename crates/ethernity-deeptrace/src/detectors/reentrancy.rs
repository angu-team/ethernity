use super::{DetectedEvent, EventSeverity, SpecializedDetector};
use crate::analyzer::TraceAnalysisResult;
use async_trait::async_trait;
use std::collections::HashMap;

/// Detector de reentrancy attacks
pub struct ReentrancyDetector;

impl ReentrancyDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SpecializedDetector for ReentrancyDetector {
    fn name(&self) -> &str {
        "ReentrancyDetector"
    }

    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut events = Vec::new();
        let mut call_stack = HashMap::new();

        analysis.call_tree.traverse_preorder(|node| {
            if let Some(to) = node.to {
                let key = (to, node.from);
                let count = call_stack.entry(key).or_insert(0);
                *count += 1;

                if *count > 1 {
                    let mut has_nested_calls = false;
                    analysis.call_tree.traverse_preorder(|other_node| {
                        if other_node.depth > node.depth && other_node.to == Some(node.from) {
                            has_nested_calls = true;
                        }
                    });

                    if has_nested_calls {
                        let mut data = serde_json::Map::new();
                        data.insert("contract".to_string(), serde_json::Value::String(format!("{:?}", to)));
                        data.insert("caller".to_string(), serde_json::Value::String(format!("{:?}", node.from)));
                        data.insert("call_count".to_string(), serde_json::Value::Number(serde_json::Number::from(*count)));

                        let event = DetectedEvent {
                            event_type: "reentrancy".to_string(),
                            confidence: 0.8,
                            addresses: vec![to, node.from],
                            data: serde_json::Value::Object(data),
                            description: "Possível reentrancy attack detectado".to_string(),
                            severity: EventSeverity::Critical,
                        };

                        events.push(event);
                    }
                }
            }
        });

        Ok(events)
    }
}
