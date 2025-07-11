use crate::detectors::clusters::uniswap_v2::analyze_uniswap_v2;
use crate::dex::{detect_swap_function, RouterInfo};
use crate::tx_logs::TxLogs;
use crate::types::{AnalysisResult, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethernity_core::traits::RpcProvider;
use ethers::abi::AbiParser;
use std::sync::Arc;

pub struct SmartRouterUniswapV3Detector;

#[async_trait]
impl crate::detectors::VictimDetector for SmartRouterUniswapV3Detector {
    fn supports(&self, _router: &RouterInfo) -> bool {
        true
    }

    async fn analyze(
        &self,
        rpc_client: Arc<dyn RpcProvider>,
        rpc_endpoint: String,
        tx: TransactionData,
        block: Option<u64>,
        outcome: TxLogs,
        router: RouterInfo,
    ) -> Result<AnalysisResult> {
        analyze_uniswap_v3(rpc_client, rpc_endpoint, tx, outcome, block, router).await
    }
}

pub async fn analyze_uniswap_v3(
    rpc_client: Arc<dyn RpcProvider>,
    rpc_endpoint: String,
    tx: TransactionData,
    outcome: TxLogs,
    block: Option<u64>,
    router: RouterInfo,
) -> Result<AnalysisResult> {
    const MULTICALL_SELECTOR: [u8; 4] = [0x5a, 0xe4, 0x01, 0xdc];
    if tx.data.len() < 4 || tx.data[..4] != MULTICALL_SELECTOR {
        return Err(anyhow!("not a multicall"));
    }

    let abi = AbiParser::default().parse_function("multicall(uint256,bytes[])")?;
    let tokens = abi.decode_input(&tx.data[4..])?;
    let calls: Vec<Vec<u8>> = tokens[1]
        .clone()
        .into_array()
        .unwrap()
        .into_iter()
        .map(|t| t.into_bytes().unwrap())
        .collect();

    for call in calls {
        if detect_swap_function(&call).is_some() {
            let mut inner = tx.clone();
            inner.data = call;
            inner.to = router.address;
            return analyze_uniswap_v2(
                rpc_client,
                rpc_endpoint,
                inner,
                outcome.logs.clone(),
                block,
                router.clone(),
            )
            .await;
        }
    }

    Err(anyhow!("no swap call found"))
}
