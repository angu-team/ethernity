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
pub mod erc721;
pub mod dex;
pub mod lending;
pub mod flash_loan;
pub mod mev;
pub mod rug_pull;
pub mod governance;

pub use erc20::Erc20PatternDetector;
pub use erc721::Erc721PatternDetector;
pub use dex::DexPatternDetector;
pub use lending::LendingPatternDetector;
pub use flash_loan::FlashLoanPatternDetector;
pub use mev::MevPatternDetector;
pub use rug_pull::RugPullPatternDetector;
pub use governance::GovernancePatternDetector;
