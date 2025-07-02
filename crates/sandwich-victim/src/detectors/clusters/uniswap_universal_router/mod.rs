use crate::dex::{RouterInfo, SwapFunction};
use crate::filters::{FilterPipeline, SwapLogFilter};
use crate::simulation::SimulationOutcome;
use crate::core::metrics::{constant_product_output, U256Ext};
use crate::types::{AnalysisResult, Metrics, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethereum_types::{Address, U256};
use ethernity_core::traits::RpcProvider;
use ethers::abi::AbiParser;
use ethers::prelude::{Http, Middleware, Provider, TransactionRequest};
use ethers::types::BlockId;
use ethers::utils::{id, keccak256};
use ethereum_types::H256;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

/// Detector for Uniswap Universal Router interactions.
pub struct UniswapUniversalRouterDetector;

static UNIVERSAL_ROUTER_ADDRESSES: Lazy<HashSet<Address>> = Lazy::new(|| {
    [
        "0x1a0a18ac4becddbd6389559687d1a73d8927e416",
        "0x1095692a6237d83c6a72f3f5efedb9a670c49223",
        "0x3a9d48ab9751398bbfa63ad67599bb04e4bdf98b",
        "0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad",
        "0x40d51104da22e3e77b683894e7e3e12e8fc61e65",
        "0x4c60051384bd2d3c01bfc845cf5f4b44bcbe9de5",
        "0x4dae2f939acf50408e13d58534ff8c2776d45265",
        "0x5302086a3a25d473aabbd0356eff8dd811a4d89b",
        "0x5e325eda8064b456f4781070c0738d849c824258",
        "0x643770e279d5d0733f21d6dc03a8efbabf3255b4",
        "0x66a9893cc07d91d95644aedd05d03f95e1dba8af",
        "0x6ff5693b99212da76ad316178a184ab56d299b43",
        "0x76d631990d505e4e5b432eedb852a60897824d68",
        "0x82635af6146972cd6601161c4472ffe97237d292",
        "0x851116d9223fabed8e56c0e6b8ad0c31d98b3507",
        "0x9e18efb3be848940b0c92d300504fb08c287fe85",
        "0xa51afafe0263b40edaef0df8781ea9aa03e381a3",
        "0xb555edf5dcf85f42ceef1f3630a52a108e55a654",
        "0xcb1355ff08ab38bbce60111f1bb2b784be25d7e8",
        "0xec7be89e9d109e7e3fec59c222cf297125fefda2",
        "0xec8b0f7ffe3ae75d7ffab09429e3675bb63503e4",
        "0xef1c6e67703c7bd7107eed8303fbe6ec2554bf6b",
        // BSC mainnet Universal Router
        "0xd9c500dff816a1da21a48a732d3498bf09dc9aeb",
    ]
    .into_iter()
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
        rpc_client: Arc<dyn RpcProvider>,
        rpc_endpoint: String,
        tx: TransactionData,
        block: Option<u64>,
        outcome: SimulationOutcome,
        _router: RouterInfo,
    ) -> Result<AnalysisResult> {
        analyze_universal_router(rpc_client, rpc_endpoint, tx, outcome, block).await
    }
}

pub async fn analyze_universal_router(
    _rpc_client: Arc<dyn RpcProvider>,
    rpc_endpoint: String,
    tx: TransactionData,
    _outcome: SimulationOutcome,
    block: Option<u64>,
) -> Result<AnalysisResult> {
    let provider = Provider::<Http>::try_from(rpc_endpoint.clone())?
        .interval(Duration::from_millis(1));
    let call_block = block.map(|b| BlockId::Number(b.into()));
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
    let inputs: Vec<Vec<u8>> = tokens
        .get(1)
        .and_then(|t| t.clone().into_array())
        .ok_or_else(|| anyhow!("missing inputs"))?
        .into_iter()
        .map(|v| v.into_bytes().unwrap_or_default())
        .collect();

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
        // attempt to decode the first swap command to extract basic info
        let mut token_route = Vec::new();
        let mut slippage = 0.0f64;
        for (idx, cmd) in commands.iter().enumerate() {
            let op = cmd & 0x3f;
            if op == 0x08 || op == 0x09 {
                // V2 swap commands
                let func_sig = if op == 0x08 {
                    "v2SwapExactInput(address,uint256,uint256,address[],address)"
                } else {
                    "v2SwapExactOutput(address,uint256,uint256,address[],address)"
                };
                let f = AbiParser::default().parse_function(func_sig)?;
                let input_data = match inputs.get(idx) {
                    Some(d) => d,
                    None => break,
                };
                let tokens = f.decode_input(input_data)?;
                let path_tokens = tokens.get(3).and_then(|t| t.clone().into_array()).ok_or_else(|| anyhow!("missing path"))?;
                let path: Vec<Address> = path_tokens
                    .into_iter()
                    .map(|t| t.into_address().unwrap())
                    .collect();
                token_route = path.clone();

                if path.len() == 2 {
                    let swap_topic: H256 = H256::from_slice(
                        keccak256("Swap(address,uint256,uint256,uint256,uint256,address)").as_slice(),
                    );
                    if let Some(log) = outcome.logs.iter().find(|l| l.topics.get(0) == Some(&swap_topic)) {
                        let pair = log.address;
                        let abi_token = AbiParser::default().parse_function("token0() view returns (address)")?;
                        let data = abi_token.encode_input(&[])?;
                        let tx_call = TransactionRequest::new().to(pair).data(data.clone());
                        let out = provider.call(&tx_call.into(), call_block).await.map_err(|e| anyhow!(e))?;
                        let token0 = abi_token.decode_output(&out)?[0].clone().into_address().unwrap();
                        let abi_token1 = AbiParser::default().parse_function("token1() view returns (address)")?;
                        let tx_call = TransactionRequest::new().to(pair).data(abi_token1.encode_input(&[])?);
                        let out1 = provider.call(&tx_call.into(), call_block).await.map_err(|e| anyhow!(e))?;
                        let token1 = abi_token1.decode_output(&out1)?[0].clone().into_address().unwrap();
                        let abi_res = AbiParser::default().parse_function("getReserves() returns (uint112,uint112,uint32)")?;
                        let tx_call = TransactionRequest::new().to(pair).data(abi_res.encode_input(&[])?);
                        let res_out = provider.call(&tx_call.into(), call_block).await.map_err(|e| anyhow!(e))?;
                        let r = abi_res.decode_output(&res_out)?;
                        let reserve0 = r[0].clone().into_uint().unwrap();
                        let reserve1 = r[1].clone().into_uint().unwrap();
                        let (reserve_in, reserve_out) = if token0 == path[0] && token1 == path[1] {
                            (reserve0, reserve1)
                        } else if token1 == path[0] && token0 == path[1] {
                            (reserve1, reserve0)
                        } else {
                            continue;
                        };
                        let amount_in = tokens
                            .get(1)
                            .map(|t| t.clone().into_uint().unwrap_or_default())
                            .unwrap_or_default();
                        let expected = constant_product_output(amount_in, reserve_in, reserve_out);
                        let transfer_sig: H256 = H256::from_slice(keccak256("Transfer(address,address,uint256)").as_slice());
                        let mut actual_out = U256::zero();
                        for log in &outcome.logs {
                            if log.topics.get(0) == Some(&transfer_sig) && log.topics.len() >= 3 {
                                let to_addr = Address::from_slice(&log.topics[2].as_bytes()[12..]);
                                if to_addr == tx.from {
                                    actual_out = U256::from_big_endian(&log.data.0);
                                }
                            }
                        }
                        if expected > actual_out && !expected.is_zero() {
                            slippage = (expected - actual_out).to_f64_lossy() / expected.to_f64_lossy();
                        }
                    }
                }
                break;
            }
        }

        let metrics = Metrics {
            swap_function: swap_variant,
            token_route,
            slippage,
            min_tokens_to_affect: U256::zero(),
            potential_profit: U256::zero(),
            router_address: tx.to,
            router_name: Some(format!("{:#x}", tx.to)),
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

