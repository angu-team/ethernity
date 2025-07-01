pub mod exact_in;
pub use exact_in::SwapV2ExactInDetector;

use crate::core::metrics::{constant_product_output, simulate_sandwich_profit, U256Ext};
use crate::dex::{detect_swap_function, get_pair_address, RouterInfo, SwapFunction};
use crate::filters::{FilterPipeline, SwapLogFilter};
use crate::simulation::{simulate_transaction, SimulationConfig, SimulationOutcome};
use crate::types::{AnalysisResult, Metrics, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethereum_types::{Address, H256, U256};
use ethernity_core::traits::RpcProvider;
use ethers::abi::{AbiParser, Token};
use ethers::prelude::{Http, Middleware, Provider, TransactionRequest};
use ethers::types::BlockId;
use ethers::utils::keccak256;
use std::sync::Arc;
use std::time::Duration;

pub struct UniswapV2Detector;

#[async_trait]
impl crate::detectors::VictimDetector for UniswapV2Detector {
    fn supports(&self, router: &RouterInfo) -> bool {
        router.factory.is_some()
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
        analyze_uniswap_v2(rpc_client, rpc_endpoint, tx, block, router).await
    }
}

pub async fn analyze_uniswap_v2(
    rpc_client: Arc<dyn RpcProvider>,
    rpc_endpoint: String,
    tx: TransactionData,
    block: Option<u64>,
    router: RouterInfo,
) -> Result<AnalysisResult> {
    let sim_config = SimulationConfig {
        rpc_endpoint,
        block_number: block,
    };

    let outcome = simulate_transaction(&sim_config, &tx).await?;
    let outcome = FilterPipeline::new()
        .push(SwapLogFilter)
        .run(outcome)
        .ok_or(anyhow!("No swap event"))?;
    let SimulationOutcome { tx_hash, logs } = outcome;

    // Use provided router information when available
    let router_address = crate::dex::router_from_logs(&logs).unwrap_or(router.address);
    let router: RouterInfo = if router_address == router.address {
        router.clone()
    } else {
        crate::dex::identify_router(&*rpc_client, router_address).await?
    };

    use std::collections::HashSet;

    let deposit_topic: H256 = H256::from_slice(keccak256("Deposit(address,uint256)").as_slice());
    let deposit_tokens: HashSet<Address> = logs
        .iter()
        .filter(|log| log.topics.get(0) == Some(&deposit_topic))
        .map(|log| log.address)
        .collect();
    let deposit_token = if deposit_tokens.len() == 1 {
        deposit_tokens.iter().next().copied()
    } else {
        None
    };

    let (swap_kind, function) =
        detect_swap_function(&tx.data).ok_or(anyhow!("unrecognized swap"))?;
    let tokens = function.decode_input(&tx.data[4..])?;

    let (amount_in, amount_out, amount_in_max, amount_out_min, path, pair_addr_opt) =
        match swap_kind {
            SwapFunction::SwapExactTokensForTokens
            | SwapFunction::SwapExactTokensForETH
            | SwapFunction::SwapExactTokensForTokensSupportingFeeOnTransferTokens
            | SwapFunction::SwapExactTokensForETHSupportingFeeOnTransferTokens => {
                let amount_in = tokens[0].clone().into_uint().unwrap();
                let amount_out_min = tokens[1].clone().into_uint().unwrap();
                let path: Vec<Address> = tokens[2]
                    .clone()
                    .into_array()
                    .unwrap()
                    .into_iter()
                    .map(|t| t.into_address().unwrap())
                    .collect();
                (
                    Some(amount_in),
                    None,
                    None,
                    Some(amount_out_min),
                    path,
                    None,
                )
            }
            SwapFunction::SwapTokensForExactTokens | SwapFunction::SwapTokensForExactETH => {
                let amount_out = tokens[0].clone().into_uint().unwrap();
                let amount_in_max = tokens[1].clone().into_uint().unwrap();
                let path: Vec<Address> = tokens[2]
                    .clone()
                    .into_array()
                    .unwrap()
                    .into_iter()
                    .map(|t| t.into_address().unwrap())
                    .collect();
                (
                    None,
                    Some(amount_out),
                    Some(amount_in_max),
                    None,
                    path,
                    None,
                )
            }
            SwapFunction::SwapExactETHForTokens
            | SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokens => {
                let amount_out_min = tokens[0].clone().into_uint().unwrap();
                let path: Vec<Address> = tokens[1]
                    .clone()
                    .into_array()
                    .unwrap()
                    .into_iter()
                    .map(|t| t.into_address().unwrap())
                    .collect();
                (Some(tx.value), None, None, Some(amount_out_min), path, None)
            }
            SwapFunction::ETHForExactTokens => {
                let amount_out = tokens[0].clone().into_uint().unwrap();
                let path: Vec<Address> = tokens[1]
                    .clone()
                    .into_array()
                    .unwrap()
                    .into_iter()
                    .map(|t| t.into_address().unwrap())
                    .collect();
                (None, Some(amount_out), Some(tx.value), None, path, None)
            }
            SwapFunction::SwapV2ExactIn => {
                let mut token_in = tokens[0].clone().into_address().unwrap();
                if token_in == Address::zero() {
                    if let Some(deposit) = deposit_token {
                        token_in = deposit;
                    }
                }
                let token_out = tokens[1].clone().into_address().unwrap();
                let amount_in = tokens[2].clone().into_uint().unwrap();
                let amount_out_min = tokens[3].clone().into_uint().unwrap();
                let pair = tokens[4].clone().into_address().unwrap();
                let path = vec![token_in, token_out];
                (
                    Some(amount_in),
                    None,
                    None,
                    Some(amount_out_min),
                    path,
                    Some(pair),
                )
            }
            _ => return Err(anyhow!("unsupported swap")),
        };

    let path_tokens: Vec<Token> = path.iter().map(|a| Token::Address(*a)).collect();

    let provider = Provider::<Http>::try_from(sim_config.rpc_endpoint.clone())?
        .interval(Duration::from_millis(1));

    let pair_address = if let Some(addr) = pair_addr_opt {
        addr
    } else if let Some(factory) = router.factory {
        get_pair_address(&*rpc_client, factory, path[0], path[1]).await?
    } else {
        return Err(anyhow!("router does not expose factory"));
    };

    let (token0, token1, reserve0, reserve1) = {
        let abi = AbiParser::default().parse_function("token0() view returns (address)")?;
        let data = abi.encode_input(&[])?;
        let tx_call = TransactionRequest::new()
            .to(pair_address)
            .data(data.clone());
        let call = provider
            .call(&tx_call.into(), block.map(|b| BlockId::Number(b.into())))
            .await
            .map_err(|e| anyhow!(e))?;
        let token0 = abi.decode_output(&call)?[0].clone().into_address().unwrap();

        let abi1 = AbiParser::default().parse_function("token1() view returns (address)")?;
        let data1 = abi1.encode_input(&[])?;
        let tx_call = TransactionRequest::new()
            .to(pair_address)
            .data(data1.clone());
        let call = provider
            .call(&tx_call.into(), block.map(|b| BlockId::Number(b.into())))
            .await
            .map_err(|e| anyhow!(e))?;
        let token1 = abi1.decode_output(&call)?[0]
            .clone()
            .into_address()
            .unwrap();

        let abi_res = AbiParser::default()
            .parse_function("getReserves() returns (uint112,uint112,uint32)")?;
        let data_res = abi_res.encode_input(&[])?;
        let tx_call = TransactionRequest::new().to(pair_address).data(data_res);
        let call = provider
            .call(&tx_call.into(), block.map(|b| BlockId::Number(b.into())))
            .await
            .map_err(|e| anyhow!(e))?;
        let tokens = abi_res.decode_output(&call)?;
        (
            token0,
            token1,
            tokens[0].clone().into_uint().unwrap(),
            tokens[1].clone().into_uint().unwrap(),
        )
    };

    let (reserve_in, reserve_out) = if token0 == path[1] {
        (reserve1, reserve0)
    } else {
        (reserve0, reserve1)
    };

    let (expected_out, expected_in) = if let Some(a_in) = amount_in {
        if pair_addr_opt.is_some() {
            (
                Some(constant_product_output(a_in, reserve_in, reserve_out)),
                None,
            )
        } else {
            let abi = AbiParser::default()
                .parse_function("getAmountsOut(uint256,address[]) returns (uint256[])")?;
            let data = abi.encode_input(&[Token::Uint(a_in), Token::Array(path_tokens.clone())])?;
            let tx_call = TransactionRequest::new()
                .to(router.address)
                .data(data.clone());
            let call = provider
                .call(&tx_call.into(), block.map(|b| BlockId::Number(b.into())))
                .await
                .map_err(|e| anyhow!(e))?;
            let out_tokens = abi.decode_output(&call)?;
            let out = out_tokens[0]
                .clone()
                .into_array()
                .unwrap()
                .last()
                .unwrap()
                .clone()
                .into_uint()
                .unwrap();
            (Some(out), None)
        }
    } else if let Some(a_out) = amount_out {
        let abi = AbiParser::default()
            .parse_function("getAmountsIn(uint256,address[]) returns (uint256[])")?;
        let data = abi.encode_input(&[Token::Uint(a_out), Token::Array(path_tokens.clone())])?;
        let tx_call = TransactionRequest::new()
            .to(router.address)
            .data(data.clone());
        let call = provider
            .call(&tx_call.into(), block.map(|b| BlockId::Number(b.into())))
            .await
            .map_err(|e| anyhow!(e))?;
        let in_tokens = abi.decode_output(&call)?;
        let inp = in_tokens[0]
            .clone()
            .into_array()
            .unwrap()
            .first()
            .unwrap()
            .clone()
            .into_uint()
            .unwrap();
        (None, Some(inp))
    } else {
        (None, None)
    };

    let transfer_sig: H256 =
        H256::from_slice(keccak256("Transfer(address,address,uint256)").as_slice());
    let mut actual_out = U256::zero();
    let mut actual_in = U256::zero();
    for log in &logs {
        if log.topics.get(0) == Some(&transfer_sig) && log.topics.len() == 3 {
            let from_addr = Address::from_slice(&log.topics[1].as_bytes()[12..]);
            let to_addr = Address::from_slice(&log.topics[2].as_bytes()[12..]);
            if to_addr == tx.from {
                actual_out = U256::from_big_endian(&log.data.0);
            }
            if from_addr == tx.from {
                actual_in = U256::from_big_endian(&log.data.0);
            }
        }
    }

    let slippage = if let Some(exp_out) = expected_out {
        if exp_out > actual_out {
            (exp_out - actual_out).to_f64_lossy() / exp_out.to_f64_lossy()
        } else {
            0.0
        }
    } else if let Some(exp_in) = expected_in {
        if actual_in > exp_in {
            (actual_in - exp_in).to_f64_lossy() / exp_in.to_f64_lossy()
        } else {
            0.0
        }
    } else {
        0.0
    };

    let min_tokens_to_affect = reserve_in / U256::from(100u64);
    let input_for_profit = amount_in.unwrap_or(actual_in);
    let potential_profit = simulate_sandwich_profit(input_for_profit, reserve_in, reserve_out);

    let router_name = router
        .name
        .clone()
        .unwrap_or_else(|| format!("{:#x}", router.address));

    let metrics = Metrics {
        swap_function: swap_kind,
        token_route: path.clone(),
        slippage,
        min_tokens_to_affect,
        potential_profit,
        router_address: router.address,
        router_name: Some(router_name),
    };

    let potential_victim = if let Some(out_min) = amount_out_min {
        slippage > 0.0 && expected_out.unwrap_or(U256::zero()) >= out_min
    } else if let Some(in_max) = amount_in_max {
        slippage > 0.0 && actual_in <= in_max
    } else {
        slippage > 0.0
    };

    Ok(AnalysisResult {
        potential_victim,
        economically_viable: potential_profit > U256::zero(),
        simulated_tx: tx_hash,
        metrics,
    })
}
