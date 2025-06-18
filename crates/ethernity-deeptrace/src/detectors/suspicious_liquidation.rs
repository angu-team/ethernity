use super::{DetectedEvent, EventSeverity, SpecializedDetector};
use crate::analyzer::TraceAnalysisResult;
use async_trait::async_trait;
use ethereum_types::U256;

/// Detector de liquidações suspeitas
pub struct SuspiciousLiquidationDetector;

impl SuspiciousLiquidationDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SpecializedDetector for SuspiciousLiquidationDetector {
    fn name(&self) -> &str {
        "SuspiciousLiquidationDetector"
    }

    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut events = Vec::new();

        for window in analysis.token_transfers.windows(3) {
            if let [transfer1, transfer2, transfer3] = window {
                if transfer1.amount > U256::from(100000) &&
                    transfer2.from != transfer1.from &&
                    transfer3.to == transfer1.from {

                    let mut data = serde_json::Map::new();
                    data.insert("liquidator".to_string(), serde_json::Value::String(format!("{:?}", transfer1.from)));
                    data.insert("victim".to_string(), serde_json::Value::String(format!("{:?}", transfer2.from)));
                    data.insert("manipulation_amount".to_string(), serde_json::Value::String(transfer1.amount.to_string()));
                    data.insert("liquidation_amount".to_string(), serde_json::Value::String(transfer2.amount.to_string()));

                    let event = DetectedEvent {
                        event_type: "suspicious_liquidation".to_string(),
                        confidence: 0.75,
                        addresses: vec![transfer1.from, transfer2.from, transfer1.token_address],
                        data: serde_json::Value::Object(data),
                        description: "Liquidação suspeita detectada".to_string(),
                        severity: EventSeverity::High,
                    };

                    events.push(event);
                }
            }
        }

        Ok(events)
    }
}
