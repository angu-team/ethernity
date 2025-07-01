use crate::dex::{RouterInfo, SwapFunction};
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
    let addrs = [
        "0x1095692a6237d83c6a72f3f5efedb9a670c49223",
        "0x112908dac86e20e7241b0927479ea3bf935d1fa0",
        "0x1906c1d672b88cd1b9ac7593301ca990f94eae07",
        "0x3315ef7ca28db74abadc6c44570efdf06b04b020",
        "0x3a9d48ab9751398bbfa63ad67599bb04e4bdf98b",
        "0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad",
        "0x4648a43b2c14da09fdf82b161150d3f634f40491",
        "0x4c60051384bd2d3c01bfc845cf5f4b44bcbe9de5",
        "0x4cded7edf52c8aa5259a54ec6a3ce7c6d2a455df",
        "0x4dae2f939acf50408e13d58534ff8c2776d45265",
        "0x5dc88340e1c5c6366864ee415d6034cadd1a9897",
        "0x5e325eda8064b456f4781070c0738d849c824258",
        "0x643770e279d5d0733f21d6dc03a8efbabf3255b4",
        "0x66a9893cc07d91d95644aedd05d03f95e1dba8af",
        "0x6ff5693b99212da76ad316178a184ab56d299b43",
        "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
        "0x82635af6146972cd6601161c4472ffe97237d292",
        "0x851116d9223fabed8e56c0e6b8ad0c31d98b3507",
        "0x8ac7bee993bb44dab564ea4bc9ea67bf9eb5e743",
        "0x95273d871c8156636e114b63797d78d7e1720d81",
        "0x986dadb82491834f6d17bd3287eb84be0b4d4cc7",
        "0xa51afafe0263b40edaef0df8781ea9aa03e381a3",
        "0xb555edf5dcf85f42ceef1f3630a52a108e55a654",
        "0xc73d61d192fb994157168fb56730fdec64c9cb8f",
        "0xcb1355ff08ab38bbce60111f1bb2b784be25d7e8",
        "0xd0872d928672ae2ff74bdb2f5130ac12229cafaf",
        "0xe463635f6e73c1e595554c3ae216472d0fb929a9",
        "0xeabbcb3e8e415306207ef514f660a3f820025be3",
        "0xec7be89e9d109e7e3fec59c222cf297125fefda2",
        "0xec8b0f7ffe3ae75d7ffab09429e3675bb63503e4",
        "0xef1c6e67703c7bd7107eed8303fbe6ec2554bf6b",
        "0xef740bf23acae26f6492b10de645d6b98dc8eaf3",
    ];
    addrs
        .into_iter()
        .map(|s| Address::from_str(s).expect("valid address"))
        .collect::<HashSet<_>>()
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
        analyze_universal_router(tx).await
    }
}

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
    let commands = tokens
        .get(0)
        .and_then(|t| t.clone().into_bytes())
        .ok_or_else(|| anyhow!("invalid commands parameter"))?;

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
        let metrics = Metrics {
            swap_function: SwapFunction::SwapV2ExactIn,
            token_route: Vec::new(),
            slippage: 0.0,
            min_tokens_to_affect: U256::zero(),
            potential_profit: U256::zero(),
            router_address: tx.to,
            router_name: Some("Universal Router".into()),
        };
        Ok(AnalysisResult {
            potential_victim: false,
            economically_viable: false,
            simulated_tx: None,
            metrics,
        })
    } else {
        Err(anyhow!("no universal router swap commands"))
    }
}

