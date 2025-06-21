use crate::{analyzer::TraceAnalysisResult, DetectedPattern, PatternType};
use async_trait::async_trait;

#[async_trait]
pub trait PatternDetector: Send + Sync {
    fn pattern_type(&self) -> PatternType;
    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()>;
    fn min_confidence(&self) -> f64 {
        0.7
    }
}

pub mod erc20;

pub use erc20::Erc20PatternDetector;
