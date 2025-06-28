use crate::dex::{identify_router, detect_swap_function, RouterInfo, SwapFunction};
use crate::simulation::{simulate_transaction, SimulationConfig, SimulationOutcome};
use crate::types::{AnalysisResult, Metrics, TransactionData};
use crate::utils::{simulate_sandwich_profit, U256Ext};
use anyhow::{anyhow, Result};
use ethers::abi::{AbiParser, Token};
use ethers::prelude::*;
use ethers::utils::keccak256;
use ethereum_types::{Address, U256, H256};
use std::time::Duration;

pub mod onchain;
use onchain::{get_pair_address, get_pair_reserves};


pub async fn analyze_transaction(rpc_endpoint: String, tx: TransactionData) -> Result<AnalysisResult> {
    let provider = Provider::<Http>::try_from(rpc_endpoint.clone())?.interval(Duration::from_millis(100));
    let router: RouterInfo = identify_router(&provider, tx.to).await?;

    let sim_config = SimulationConfig {
        rpc_endpoint,
        block_number: None,
    };
    let SimulationOutcome { tx_hash, logs } = simulate_transaction(&sim_config, &tx).await?;

    let (swap_kind, function) = detect_swap_function(&tx.data)
        .ok_or_else(|| anyhow!("função swap não reconhecida"))?;
    let tokens = function.decode_input(&tx.data[4..])?;

    let (amount_in, amount_out, amount_in_max, amount_out_min, path) = match swap_kind {
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
            (Some(amount_in), None, None, Some(amount_out_min), path)
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
            (None, Some(amount_out), Some(amount_in_max), None, path)
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
            (Some(tx.value), None, None, Some(amount_out_min), path)
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
            (None, Some(amount_out), Some(tx.value), None, path)
        }
    };

    let path_tokens: Vec<Token> = path.iter().map(|a| Token::Address(*a)).collect();
    let (expected_out, expected_in) = if let Some(a_in) = amount_in {
        let abi = AbiParser::default()
            .parse_function("getAmountsOut(uint256,address[]) returns (uint256[])")?;
        let data = abi.encode_input(&[Token::Uint(a_in), Token::Array(path_tokens.clone())])?;
        let req = ethers::types::TransactionRequest {
            to: Some(NameOrAddress::Address(tx.to)),
            data: Some(data.into()),
            ..Default::default()
        };
        let call = provider.call(&req.into(), None).await?;
        let out_tokens = abi.decode_output(&call.0)?;
        let out = out_tokens[0].clone().into_array().unwrap().last().unwrap().clone().into_uint().unwrap();
        (Some(out), None)
    } else if let Some(a_out) = amount_out {
        let abi = AbiParser::default()
            .parse_function("getAmountsIn(uint256,address[]) returns (uint256[])")?;
        let data = abi.encode_input(&[Token::Uint(a_out), Token::Array(path_tokens.clone())])?;
        let req = ethers::types::TransactionRequest {
            to: Some(NameOrAddress::Address(tx.to)),
            data: Some(data.into()),
            ..Default::default()
        };
        let call = provider.call(&req.into(), None).await?;
        let in_tokens = abi.decode_output(&call.0)?;
        let inp = in_tokens[0].clone().into_array().unwrap().first().unwrap().clone().into_uint().unwrap();
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

    let pair_address = if let Some(factory) = router.factory {
        get_pair_address(&provider, factory, path[0], path[1]).await?
    } else {
        return Err(anyhow!("router não fornece fábrica"));
    };

    let (reserve_in, reserve_out) = get_pair_reserves(&provider, pair_address).await?;
    let min_tokens_to_affect = reserve_in / U256::from(100u64);
    let input_for_profit = amount_in.unwrap_or(actual_in);
    let potential_profit = simulate_sandwich_profit(input_for_profit, reserve_in, reserve_out);

    let metrics = Metrics {
        swap_function: swap_kind,
        token_route: path.clone(),
        slippage,
        min_tokens_to_affect,
        potential_profit,
        router_address: router.address,
        router_name: router.name.clone(),
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
