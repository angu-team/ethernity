use crate::dex::RouterInfo;
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, TransactionData};
use anyhow::Result;
use async_trait::async_trait;
use ethernity_core::traits::RpcProvider;
use std::sync::Arc;

pub mod clusters;
use clusters::uniswap_v2::{UniswapV2Detector, SwapV2ExactInDetector};
use clusters::uniswap_v3::UniswapV3Detector;
use clusters::uniswap_v4::UniswapV4Detector;
use clusters::smart_router::MulticallBytesDetector;
use clusters::smart_router::custom::SmartRouterUniswapV3Detector;
use clusters::oneinch_generic_router::OneInchGenericRouterDetector;

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
            detectors: vec![
                Box::new(UniswapV3Detector),
                Box::new(SmartRouterUniswapV3Detector),
                Box::new(MulticallBytesDetector),
                Box::new(OneInchGenericRouterDetector),
                Box::new(UniswapV4Detector),
                Box::new(UniswapV2Detector),
                Box::new(SwapV2ExactInDetector),
            ],
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
        let mut last_err = None;
        for d in &self.detectors {
            if d.supports(&router) {
                match d
                    .analyze(
                        rpc_client.clone(),
                        rpc_endpoint.clone(),
                        tx.clone(),
                        block,
                        outcome.clone(),
                        router.clone(),
                    )
                    .await
                {
                    Ok(res) => return Ok(res),
                    Err(e) => last_err = Some(e),
                }
            }
        }
        if let Some(err) = last_err {
            Err(err)
        } else {
            Err(anyhow::anyhow!("unsupported router"))
        }
    }
}
