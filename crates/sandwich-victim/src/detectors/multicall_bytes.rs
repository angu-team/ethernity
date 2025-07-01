use crate::detectors::uniswap_v2::analyze_uniswap_v2;
use crate::dex::{detect_swap_function, RouterInfo};
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethernity_core::traits::RpcProvider;
use ethers::abi::AbiParser;
use std::sync::Arc;

/// Detector para a função `multicall(bytes[])`.
pub struct MulticallBytesDetector;

#[async_trait]
impl crate::detectors::VictimDetector for MulticallBytesDetector {
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
        analyze_multicall_bytes(rpc_client, rpc_endpoint, tx, block, router).await
    }
}

pub async fn analyze_multicall_bytes(
    rpc_client: Arc<dyn RpcProvider>,
    rpc_endpoint: String,
    tx: TransactionData,
    block: Option<u64>,
    router: RouterInfo,
) -> Result<AnalysisResult> {
    const MULTICALL_SELECTOR: [u8; 4] = [0xac, 0x96, 0x50, 0xd8];
    if tx.data.len() < 4 || tx.data[..4] != MULTICALL_SELECTOR {
        return Err(anyhow!("not a multicall"));
    }

    let abi = AbiParser::default().parse_function("multicall(bytes[])")?;
    let tokens = abi.decode_input(&tx.data[4..])?;
    let arr = tokens
        .get(0)
        .cloned()
        .ok_or_else(|| anyhow!("invalid multicall"))?
        .into_array()
        .ok_or_else(|| anyhow!("invalid array"))?;
    let mut calls: Vec<Vec<u8>> = Vec::with_capacity(arr.len());
    for t in arr {
        calls.push(t.into_bytes().ok_or_else(|| anyhow!("invalid inner calldata"))?);
    }

    let mut last_err = None;
    for call in calls {
        if detect_swap_function(&call).is_some() {
            let mut inner = tx.clone();
            inner.data = call.clone();
            let res = analyze_uniswap_v2(
                rpc_client.clone(),
                rpc_endpoint.clone(),
                inner,
                block,
            )
            .await;
            match res {
                Ok(v) => return Ok(v),
                Err(e) => last_err = Some(e),
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow!("no swap call found")))
}
