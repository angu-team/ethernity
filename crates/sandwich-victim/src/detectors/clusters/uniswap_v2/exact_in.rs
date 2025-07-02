use crate::detectors::clusters::uniswap_v2::analyze_uniswap_v2;
use crate::dex::{detect_swap_function, RouterInfo};
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethernity_core::traits::RpcProvider;
use std::sync::Arc;

pub struct SwapV2ExactInDetector;

#[async_trait]
impl crate::detectors::VictimDetector for SwapV2ExactInDetector {
    fn supports(&self, router: &RouterInfo) -> bool {
        router.factory.is_none()
    }

    async fn analyze(
        &self,
        rpc_client: Arc<dyn RpcProvider>,
        rpc_endpoint: String,
        tx: TransactionData,
        block: Option<u64>,
        _outcome: SimulationOutcome,
        router: RouterInfo,
    ) -> Result<AnalysisResult> {
        let (kind, _) = detect_swap_function(&tx.data).ok_or(anyhow!("unrecognized swap"))?;
        // Accept any UniswapV2 compatible swap when the router does not expose a factory
        if crate::detectors::clusters::Cluster::from(&kind) != crate::detectors::clusters::Cluster::UniswapV2 {
            return Err(anyhow!("unsupported swap"));
        }

        analyze_uniswap_v2(rpc_client, rpc_endpoint, tx, block, router).await
    }
}

