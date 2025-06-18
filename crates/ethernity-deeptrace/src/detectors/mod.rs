use crate::analyzer::TraceAnalysisResult;
use async_trait::async_trait;
use ethereum_types::Address;

pub mod sandwich_attack;
pub mod frontrunning;
pub mod reentrancy;
pub mod price_manipulation;
pub mod suspicious_liquidation;

pub use sandwich_attack::SandwichAttackDetector;
pub use frontrunning::FrontrunningDetector;
pub use reentrancy::ReentrancyDetector;
pub use price_manipulation::PriceManipulationDetector;
pub use suspicious_liquidation::SuspiciousLiquidationDetector;

#[async_trait]
pub trait SpecializedDetector: Send + Sync {
    fn name(&self) -> &str;
    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()>;
}

#[derive(Debug, Clone)]
pub struct DetectedEvent {
    pub event_type: String,
    pub confidence: f64,
    pub addresses: Vec<Address>,
    pub data: serde_json::Value,
    pub description: String,
    pub severity: EventSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSeverity {
    Low,
    Medium,
    High,
    Critical,
}

pub struct DetectorManager {
    detectors: Vec<Box<dyn SpecializedDetector>>,
}

impl DetectorManager {
    pub fn new() -> Self {
        let mut detectors: Vec<Box<dyn SpecializedDetector>> = Vec::new();

        detectors.push(Box::new(SandwichAttackDetector::new()));
        detectors.push(Box::new(FrontrunningDetector::new()));
        detectors.push(Box::new(ReentrancyDetector::new()));
        detectors.push(Box::new(PriceManipulationDetector::new()));
        detectors.push(Box::new(SuspiciousLiquidationDetector::new()));

        Self { detectors }
    }

    pub fn add_detector(&mut self, detector: Box<dyn SpecializedDetector>) {
        self.detectors.push(detector);
    }

    pub async fn detect_all(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut all_events = Vec::new();

        for detector in &self.detectors {
            match detector.detect_events(analysis).await {
                Ok(mut events) => all_events.append(&mut events),
                Err(e) => eprintln!("Erro no detector {}: {:?}", detector.name(), e),
            }
        }

        Ok(all_events)
    }

    pub fn available_detectors(&self) -> Vec<&str> {
        self.detectors.iter().map(|d| d.name()).collect()
    }
}

impl Default for DetectorManager {
    fn default() -> Self {
        Self::new()
    }
}
