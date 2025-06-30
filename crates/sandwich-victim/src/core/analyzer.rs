use crate::detectors::{DetectorRegistry};
use crate::dex::{identify_router, router_from_logs, RouterInfo};
use crate::filters::{FilterPipeline, SwapLogFilter};
use crate::simulation::{simulate_transaction, SimulationConfig};
use crate::types::{AnalysisResult, TransactionData};
use anyhow::{Result, anyhow};
use ethernity_core::traits::RpcProvider;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
enum AnalysisError {
    #[error("No swap event found")]
    NoSwapEvent,
    #[error("Router not found in logs")]
    NoRouterFound,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub async fn analyze_transaction(
    rpc_client: Arc<dyn RpcProvider>,
    rpc_endpoint: String,
    tx: TransactionData,
    block: Option<u64>,
) -> Result<AnalysisResult> {
    let sim_config = SimulationConfig {
        rpc_endpoint: rpc_endpoint.clone(),
        block_number: block,
    };

    let outcome = simulate_transaction(&sim_config, &tx).await?;
    let outcome = FilterPipeline::new()
        .push(SwapLogFilter)
        .run(outcome)
        .ok_or(AnalysisError::NoSwapEvent)?;

    let router_address = router_from_logs(&outcome.logs).ok_or(AnalysisError::NoRouterFound)?;
    let router: RouterInfo = identify_router(&*rpc_client, router_address).await?;

    let registry = DetectorRegistry::default();
    registry
        .analyze(rpc_client, rpc_endpoint, tx, block, outcome, router)
        .await
        .map_err(|e| anyhow!(e))
}

