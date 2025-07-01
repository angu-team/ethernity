use crate::dex::RouterInfo;
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethernity_core::traits::RpcProvider;
use std::sync::Arc;

/// Detector para funções do Uniswap V3 Router.
pub struct UniswapV3Detector;

#[async_trait]
impl crate::detectors::VictimDetector for UniswapV3Detector {
    fn supports(&self, router: &RouterInfo) -> bool {
        router.factory.is_none()
    }

    async fn analyze(
        &self,
        _rpc_client: Arc<dyn RpcProvider>,
        _rpc_endpoint: String,
        _tx: TransactionData,
        _block: Option<u64>,
        _outcome: SimulationOutcome,
        _router: RouterInfo,
    ) -> Result<AnalysisResult> {
        Err(anyhow!("uniswap v3 detector not implemented"))
    }
}
