use crate::dex::RouterInfo;
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethernity_core::traits::RpcProvider;
use ethers::abi::AbiParser;
use ethers::utils::id;
use std::sync::Arc;

/// Detector for Uniswap Universal Router interactions.
/// Currently only checks if the `execute` function contains swap-related commands
/// and returns a placeholder error.
pub struct UniswapUniversalRouterDetector;

#[async_trait]
impl crate::detectors::VictimDetector for UniswapUniversalRouterDetector {
    fn supports(&self, _router: &RouterInfo) -> bool {
        true
    }

    async fn analyze(
        &self,
        _rpc_client: Arc<dyn RpcProvider>,
        _rpc_endpoint: String,
        tx: TransactionData,
        _block: Option<u64>,
        _outcome: SimulationOutcome,
        _router: RouterInfo,
    ) -> Result<AnalysisResult> {
        analyze_universal_router(tx).await
    }
}

/// Attempt to identify Universal Router swaps.
pub async fn analyze_universal_router(tx: TransactionData) -> Result<AnalysisResult> {
    let execute_selector = &id("execute(bytes,bytes[])")[..4];
    let execute_deadline_selector = &id("execute(bytes,bytes[],uint256)")[..4];

    if tx.data.len() < 4 {
        return Err(anyhow!("not universal router"));
    }

    let abi_sig = if tx.data[..4] == execute_selector[..] {
        "execute(bytes,bytes[])"
    } else if tx.data[..4] == execute_deadline_selector[..] {
        "execute(bytes,bytes[],uint256)"
    } else {
        return Err(anyhow!("not universal router"));
    };

    let abi = AbiParser::default().parse_function(abi_sig)?;
    let tokens = abi.decode_input(&tx.data[4..])?;
    let commands = tokens[0].clone().into_bytes().unwrap_or_default();

    const SWAP_OPS: [u8; 10] = [
        0x00, // V3_SWAP_EXACT_IN
        0x01, // V3_SWAP_EXACT_OUT
        0x08, // V2_SWAP_EXACT_IN
        0x09, // V2_SWAP_EXACT_OUT
        0x02, // PERMIT2_TRANSFER_FROM
        0x0b, // WRAP_ETH
        0x0c, // UNWRAP_WETH
        0x04, // SWEEP
        0x05, // TRANSFER
        0x06, // PAY_PORTION
    ];

    if commands.iter().any(|c| SWAP_OPS.contains(c)) {
        return Err(anyhow!("uniswap universal router detector not implemented"));
    }

    Err(anyhow!("no universal router swap commands"))
}
