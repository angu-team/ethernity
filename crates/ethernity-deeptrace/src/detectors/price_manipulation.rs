use super::{DetectedEvent, EventSeverity, SpecializedDetector};
use crate::analyzer::TraceAnalysisResult;
use async_trait::async_trait;
use ethereum_types::U256;

/// Detector de price manipulation
pub struct PriceManipulationDetector;

impl PriceManipulationDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SpecializedDetector for PriceManipulationDetector {
    fn name(&self) -> &str {
        "PriceManipulationDetector"
    }

    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut events = Vec::new();

        for transfer in &analysis.token_transfers {
            if transfer.amount > U256::from(1000000) {
                let mut related_transfers = Vec::new();
                for other_transfer in &analysis.token_transfers {
                    if other_transfer.token_address == transfer.token_address &&
                        other_transfer != transfer &&
                        (other_transfer.from == transfer.to || other_transfer.to == transfer.from) {
                        related_transfers.push(other_transfer);
                    }
                }

                if related_transfers.len() >= 2 {
                    let mut data = serde_json::Map::new();
                    data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", transfer.token_address)));
                    data.insert("manipulator".to_string(), serde_json::Value::String(format!("{:?}", transfer.from)));
                    data.insert("amount".to_string(), serde_json::Value::String(transfer.amount.to_string()));
                    data.insert("related_transfers".to_string(), serde_json::Value::Number(serde_json::Number::from(related_transfers.len())));

                    let event = DetectedEvent {
                        event_type: "price_manipulation".to_string(),
                        confidence: 0.7,
                        addresses: vec![transfer.token_address, transfer.from, transfer.to],
                        data: serde_json::Value::Object(data),
                        description: "Possível manipulação de preço detectada".to_string(),
                        severity: EventSeverity::High,
                    };

                    events.push(event);
                }
            }
        }

        Ok(events)
    }
}
