use crate::dex::{RouterInfo, SwapFunction};
use crate::filters::{FilterPipeline, SwapLogFilter};
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, Metrics, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethereum_types::{Address, U256};
use ethernity_core::traits::RpcProvider;
use ethers::abi::AbiParser;
use ethers::utils::id;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;

/// Detector for Uniswap Universal Router interactions.
pub struct UniswapUniversalRouterDetector;

static UNIVERSAL_ROUTER_ADDRESSES: Lazy<HashSet<Address>> = Lazy::new(|| {
    [
        // mainnet
        "0xEf1c6E67703c7BD7107eed8303Fbe6EC2554BF6B",
        "0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD",
        "0x66a9893cc07d91d95644aedd05d03f95e1dba8af",
        // sepolia
        "0x5302086A3a25d473aAbBd0356eFf8Dd811a4d89B",
        "0x3a9d48ab9751398bbfa63ad67599bb04e4bdf98b",
        // arbitrum
        "0x4C60051384bd2d3C01bfc845Cf5F4b44bcbE9de5",
        "0xeC8B0F7Ffe3ae75d7FfAb09429e3675bb63503e4",
        "0x5E325eDA8064b456f4781070C0738d849c824258",
        "0xa51afafe0263b40edaef0df8781ea9aa03e381a3",
        // optimism
        "0xb555edF5dcF85f42cEeF1f3630a52A108E55A654",
        "0xCb1355ff08Ab38bBCE60111F1bb2B784bE25D7e8",
        "0x851116d9223fabed8e56c0e6b8ad0c31d98b3507",
        // polygon
        "0x643770E279d5D0733F21d6DC03A8efbABf3255B4",
        "0xec7BE89e9d109e7e3Fec59c222CF297125FEFda2",
        "0x1095692a6237d83c6a72f3f5efedb9a670c49223",
        // base
        "0x6ff5693b99212da76ad316178a184ab56d299b43",
    ]
    .iter()
    .map(|s| Address::from_str(s).expect("valid address"))
    .collect()
});


#[async_trait]
impl crate::detectors::VictimDetector for UniswapUniversalRouterDetector {
    fn supports(&self, router: &RouterInfo) -> bool {
        UNIVERSAL_ROUTER_ADDRESSES.contains(&router.address)
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

