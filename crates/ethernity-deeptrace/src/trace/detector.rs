use async_trait::async_trait;

use super::CallTrace;

/// Detector de padrões em traces
#[async_trait]
pub trait TraceDetector: Send + Sync {
    /// Detecta padrões em um trace
    async fn detect(&self, trace: &CallTrace) -> Result<Vec<crate::DetectedPattern>, ()>;
}
