use crate::detectors::clusters::uniswap_v2::analyze_uniswap_v2;
use crate::dex::RouterInfo;
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, TransactionData};
use anyhow::Result;
use async_trait::async_trait;
use ethereum_types::Address;
use ethernity_core::traits::RpcProvider;
use once_cell::sync::Lazy;
use std::str::FromStr;
use std::sync::Arc;

/// Detector para o Generic Router da 1inch.
pub struct OneInchGenericRouterDetector;

static GENERIC_ROUTER_ADDRESS: Lazy<Address> = Lazy::new(|| {
    Address::from_str("0x1111111254eeb25477b68fb85ed929f73a960582").expect("valid address")
});

#[async_trait]
impl crate::detectors::VictimDetector for OneInchGenericRouterDetector {
    fn supports(&self, router: &RouterInfo) -> bool {
        router.address == *GENERIC_ROUTER_ADDRESS
    }

    async fn analyze(
        &self,
        rpc_client: Arc<dyn RpcProvider>,
        rpc_endpoint: String,
        tx: TransactionData,
        block: Option<u64>,
        outcome: SimulationOutcome,
        router: RouterInfo,
    ) -> Result<AnalysisResult> {
        analyze_oneinch_generic_router(rpc_client, rpc_endpoint, tx, block, outcome, router).await
    }
}

pub async fn analyze_oneinch_generic_router(
    rpc_client: Arc<dyn RpcProvider>,
    rpc_endpoint: String,
    tx: TransactionData,
    block: Option<u64>,
    _outcome: SimulationOutcome,
    router: RouterInfo,
) -> Result<AnalysisResult> {
    analyze_uniswap_v2(rpc_client, rpc_endpoint, tx, block, router).await
}
