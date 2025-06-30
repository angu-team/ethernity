use crate::dex::RouterInfo;
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, TransactionData};
use anyhow::Result;
use async_trait::async_trait;
use ethernity_core::traits::RpcProvider;
use std::sync::Arc;

pub mod uniswap_v2;
use uniswap_v2::UniswapV2Detector;
pub mod pancakeswap_v3;
use pancakeswap_v3::PancakeSwapV3Detector;

#[async_trait]
pub trait VictimDetector: Send + Sync {
    fn supports(&self, router: &RouterInfo) -> bool;
    async fn analyze(
        &self,
        rpc_client: Arc<dyn RpcProvider>,
        rpc_endpoint: String,
        tx: TransactionData,
        block: Option<u64>,
        outcome: SimulationOutcome,
        router: RouterInfo,
    ) -> Result<AnalysisResult>;
}

pub struct DetectorRegistry {
    detectors: Vec<Box<dyn VictimDetector>>,
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        Self {
            detectors: vec![Box::new(UniswapV2Detector), Box::new(PancakeSwapV3Detector)],
        }
    }
}

impl DetectorRegistry {
    pub async fn analyze(
        &self,
        rpc_client: Arc<dyn RpcProvider>,
        rpc_endpoint: String,
        tx: TransactionData,
        block: Option<u64>,
        outcome: SimulationOutcome,
        router: RouterInfo,
    ) -> Result<AnalysisResult> {
        for d in &self.detectors {
            if d.supports(&router) {
                return d
                    .analyze(
                        rpc_client.clone(),
                        rpc_endpoint.clone(),
                        tx.clone(),
                        block,
                        outcome.clone(),
                        router.clone(),
                    )
                    .await;
            }
        }
        Err(anyhow::anyhow!("unsupported router"))
    }
}
