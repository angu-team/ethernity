use crate::dex::{RouterInfo, SwapFunction};
use crate::filters::{FilterPipeline, SwapLogFilter};
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, Metrics, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethereum_types::U256;
use ethernity_core::traits::RpcProvider;
use ethers::abi::AbiParser;
use ethers::utils::id;
use std::sync::Arc;

/// Detector for Uniswap Universal Router interactions.
pub struct UniswapUniversalRouterDetector;


#[async_trait]
impl crate::detectors::VictimDetector for UniswapUniversalRouterDetector {
    fn supports(&self, router: &RouterInfo) -> bool {
        router.factory.is_none()
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
        analyze_universal_router(tx, _outcome).await
    }
}

pub async fn analyze_universal_router(
    tx: TransactionData,
    _outcome: SimulationOutcome,
) -> Result<AnalysisResult> {
    let outcome = FilterPipeline::new()
        .push(SwapLogFilter)
        .run(_outcome)
        .ok_or(anyhow!("No swap event"))?;
    let execute_selector = &id("execute(bytes,bytes[])")[..4];
    let execute_deadline_selector = &id("execute(bytes,bytes[],uint256)")[..4];

    if tx.data.len() < 4 {
        return Err(anyhow!("not universal router"));
    }

    let (abi_sig, swap_variant) = if tx.data[..4] == execute_selector[..] {
        ("execute(bytes,bytes[])", SwapFunction::UniversalRouterSwap)
    } else if tx.data[..4] == execute_deadline_selector[..] {
        (
            "execute(bytes,bytes[],uint256)",
            SwapFunction::UniversalRouterSwapDeadline,
        )
    } else {
        return Err(anyhow!("not universal router"));
    };

    let abi = AbiParser::default().parse_function(abi_sig)?;
    let tokens = abi.decode_input(&tx.data[4..])?;
    let commands = tokens
        .get(0)
        .and_then(|t| t.clone().into_bytes())
        .ok_or_else(|| anyhow!("invalid commands parameter"))?;

    const SWAP_OPS: [u8; 5] = [
        0x00, // V3_SWAP_EXACT_IN
        0x01, // V3_SWAP_EXACT_OUT
        0x08, // V2_SWAP_EXACT_IN
        0x09, // V2_SWAP_EXACT_OUT
        0x10, // V4_SWAP
    ];

    let has_swap = commands
        .iter()
        .map(|c| c & 0x3f)
        .any(|c| SWAP_OPS.contains(&c));

    if has_swap {
        let metrics = Metrics {
            swap_function: swap_variant,
            token_route: Vec::new(),
            slippage: 0.0,
            min_tokens_to_affect: U256::zero(),
            potential_profit: U256::zero(),
            router_address: tx.to,
            router_name: Some("Universal Router".into()),
        };
        Ok(AnalysisResult {
            potential_victim: true,
            economically_viable: false,
            simulated_tx: None,
            metrics,
        })
    } else {
        Err(anyhow!("no universal router swap commands"))
    }
}

