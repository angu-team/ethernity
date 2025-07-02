use crate::core::metrics::U256Ext;
use crate::dex::{
    detect_swap_function, quote_exact_input, quote_exact_output, RouterInfo, SwapFunction,
};
use crate::filters::{FilterPipeline, SwapLogFilter};
use crate::simulation::{simulate_transaction, SimulationConfig, SimulationOutcome};
use crate::types::{AnalysisResult, Metrics, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethernity_core::traits::RpcProvider;
use ethers::abi::AbiParser;
use ethers::types::{Address, H256, U256};
use ethers::utils::keccak256;
use std::env;
use std::str::FromStr;
use std::sync::Arc;

/// Default Uniswap V3 Quoter address on Ethereum mainnet.
const DEFAULT_QUOTER: &str = "0xb27308f9f90d607463bb33ea1bebb41c27ce5ab6";

fn quoter_address() -> Address {
    env::var("UNISWAP_V3_QUOTER")
        .ok()
        .and_then(|v| Address::from_str(&v).ok())
        .unwrap_or_else(|| Address::from_str(DEFAULT_QUOTER).expect("default quoter"))
}

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
        _outcome: SimulationOutcome,
        router: RouterInfo,
    ) -> Result<AnalysisResult> {
        analyze_uniswap_v3(rpc_client, rpc_endpoint, tx, block, router).await
    }
}

pub async fn analyze_uniswap_v3(
    rpc_client: Arc<dyn RpcProvider>,
    rpc_endpoint: String,
    tx: TransactionData,
    block: Option<u64>,
    router: RouterInfo,
) -> Result<AnalysisResult> {
    const MULTICALL_DEADLINE_SELECTOR: [u8; 4] = [0x5a, 0xe4, 0x01, 0xdc];
    const MULTICALL_BYTES_SELECTOR: [u8; 4] = [0xac, 0x96, 0x50, 0xd8];

    if tx.data.len() < 4 {
        return Err(anyhow!("not a multicall"));
    }

    let calls = if tx.data[..4] == MULTICALL_DEADLINE_SELECTOR {
        let abi = AbiParser::default().parse_function("multicall(uint256,bytes[])")?;
        let tokens = abi.decode_input(&tx.data[4..])?;
        tokens[1]
            .clone()
            .into_array()
            .unwrap()
            .into_iter()
            .map(|t| t.into_bytes().unwrap())
            .collect::<Vec<_>>()
    } else if tx.data[..4] == MULTICALL_BYTES_SELECTOR {
        let abi = AbiParser::default().parse_function("multicall(bytes[])")?;
        let tokens = abi.decode_input(&tx.data[4..])?;
        tokens[0]
            .clone()
            .into_array()
            .unwrap()
            .into_iter()
            .map(|t| t.into_bytes().unwrap())
            .collect::<Vec<_>>()
    } else {
        return Err(anyhow!("not a multicall"));
    };

    for call in calls {
        if let Some((kind, function)) = detect_swap_function(&call) {
            if crate::detectors::clusters::Cluster::from(&kind)
                != crate::detectors::clusters::Cluster::UniswapV3
            {
                continue;
            }

            let tokens = function.decode_input(&call[4..])?;
            let (amount_in, amount_out, path_bytes, route_tokens) = match kind {
                SwapFunction::ExactInputSingle => {
                    let tup = tokens[0].clone().into_tuple().unwrap();
                    let token_in = tup[0].clone().into_address().unwrap();
                    let token_out = tup[1].clone().into_address().unwrap();
                    let fee = tup[2].clone().into_uint().unwrap().as_u32();
                    let amount_in = tup[5].clone().into_uint().unwrap();
                    let mut bytes = Vec::new();
                    bytes.extend_from_slice(token_in.as_bytes());
                    bytes.extend_from_slice(&fee.to_be_bytes()[1..]);
                    bytes.extend_from_slice(token_out.as_bytes());
                    (Some(amount_in), None, bytes, vec![token_in, token_out])
                }
                SwapFunction::ExactInput => {
                    let path: Vec<u8> = tokens[0].clone().into_bytes().unwrap();
                    let amount_in = tokens[3].clone().into_uint().unwrap();
                    let route_tokens = decode_path(&path);
                    (Some(amount_in), None, path, route_tokens)
                }
                SwapFunction::ExactOutputSingle => {
                    let tup = tokens[0].clone().into_tuple().unwrap();
                    let token_in = tup[0].clone().into_address().unwrap();
                    let token_out = tup[1].clone().into_address().unwrap();
                    let fee = tup[2].clone().into_uint().unwrap().as_u32();
                    let amount_out = tup[5].clone().into_uint().unwrap();
                    let mut bytes = Vec::new();
                    bytes.extend_from_slice(token_in.as_bytes());
                    bytes.extend_from_slice(&fee.to_be_bytes()[1..]);
                    bytes.extend_from_slice(token_out.as_bytes());
                    (None, Some(amount_out), bytes, vec![token_in, token_out])
                }
                SwapFunction::ExactOutput => {
                    let path: Vec<u8> = tokens[0].clone().into_bytes().unwrap();
                    let amount_out = tokens[3].clone().into_uint().unwrap();
                    let route_tokens = decode_path(&path);
                    (None, Some(amount_out), path, route_tokens)
                }
                _ => continue,
            };

            let sim_config = SimulationConfig {
                rpc_endpoint: rpc_endpoint.clone(),
                block_number: block,
            };
            let outcome = simulate_transaction(&sim_config, &tx).await?;
            let outcome = FilterPipeline::new()
                .push(SwapLogFilter)
                .run(outcome)
                .ok_or(anyhow!("No swap event"))?;
            let SimulationOutcome { tx_hash, logs } = outcome;

            let router_address = crate::dex::router_from_logs(&logs).unwrap_or(router.address);
            let router: RouterInfo = if router_address == router.address {
                router.clone()
            } else {
                crate::dex::identify_router(&*rpc_client, router_address).await?
            };

            let quoter = quoter_address();
            let (expected_out, expected_in) = if let Some(a_in) = amount_in {
                (
                    Some(quote_exact_input(&*rpc_client, quoter, path_bytes.clone(), a_in).await?),
                    None,
                )
            } else if let Some(a_out) = amount_out {
                (
                    None,
                    Some(
                        quote_exact_output(&*rpc_client, quoter, path_bytes.clone(), a_out).await?,
                    ),
                )
            } else {
                (None, None)
            };

            let transfer_sig: H256 =
                H256::from_slice(keccak256("Transfer(address,address,uint256)").as_slice());
            // Accumulate all transfers to and from the user to capture the
            // total amounts swapped in multi-hop transactions.
            let mut actual_out = U256::zero();
            let mut actual_in = U256::zero();
            for log in &logs {
                if log.topics.get(0) == Some(&transfer_sig) && log.topics.len() == 3 {
                    let from_addr = Address::from_slice(&log.topics[1].as_bytes()[12..]);
                    let to_addr = Address::from_slice(&log.topics[2].as_bytes()[12..]);
                    let amount = U256::from_big_endian(&log.data.0);
                    if to_addr == tx.from {
                        actual_out += amount;
                    }
                    if from_addr == tx.from {
                        actual_in += amount;
                    }
                }
            }

            // Guard against divisions where the f64 conversion of the expected
            // amount is zero (which can happen for extremely large values).
            let slippage = if let Some(exp_out) = expected_out {
                if exp_out > actual_out {
                    let denom = exp_out.to_f64_lossy();
                    if denom > 0.0 {
                        (exp_out - actual_out).to_f64_lossy() / denom
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else if let Some(exp_in) = expected_in {
                if actual_in > exp_in {
                    let denom = exp_in.to_f64_lossy();
                    if denom > 0.0 {
                        (actual_in - exp_in).to_f64_lossy() / denom
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let metrics = Metrics {
                swap_function: kind,
                token_route: route_tokens,
                slippage,
                min_tokens_to_affect: U256::zero(),
                potential_profit: U256::zero(),
                router_address,
                router_name: Some(router.name.clone().unwrap_or_else(|| "Smart Router".into())),
            };

            return Ok(AnalysisResult {
                potential_victim: slippage > 0.0,
                economically_viable: false,
                simulated_tx: tx_hash,
                metrics,
            });
        }
    }

    Err(anyhow!("no swap call found"))
}

fn decode_path(path: &[u8]) -> Vec<Address> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i + 20 <= path.len() {
        tokens.push(Address::from_slice(&path[i..i + 20]));
        i += 20;
        if i + 3 > path.len() {
            break;
        }
        i += 3;
    }
    tokens
}
