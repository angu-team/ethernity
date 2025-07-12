use crate::dex::{detect_swap_function, RouterInfo, SwapFunction};
use crate::tx_logs::TxLogs;
use crate::types::{AnalysisResult, Metrics, TransactionData};
use anyhow::Result;
use async_trait::async_trait;
use ethereum_types::{Address, U256};
use ethernity_core::traits::RpcProvider;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;

/// Detector para o Aggregation Router V6 da 1inch.
pub struct OneInchAggregationRouterV6Detector;

pub static AGGREGATION_ROUTER_V6_ADDRESSES: Lazy<HashSet<Address>> = Lazy::new(|| {
    [
        "0x1111111254fb6c44bac0bed2854e76f90643097d",
        "0x1111111254eeb25477b68fb85ed929f73a960582",
        "0x111111125421ca6dc452d289314280a0f8842a65",
        "0xde9e4fe32b049f821c7f3e9802381aa470ffca73",
    ]
    .into_iter()
    .map(|s| Address::from_str(s).expect("valid address"))
    .collect()
});

#[async_trait]
impl crate::detectors::VictimDetector for OneInchAggregationRouterV6Detector {
    fn supports(&self, router: &RouterInfo) -> bool {
        AGGREGATION_ROUTER_V6_ADDRESSES.contains(&router.address)
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
        analyze_oneinch_aggregation_router_v6(rpc_client, rpc_endpoint, tx, block, outcome, router)
            .await
    }
}

/// Lista de seletores de funções de swap do Aggregation Router V6.
static SWAP_SELECTORS: Lazy<Vec<[u8; 4]>> = Lazy::new(|| {
    use ethers::utils::id;
    vec![
        id("swap(address,(address,address,address,address,uint256,uint256,uint256,uint256),bytes)")[..4].try_into().unwrap(),
        id("unoswap(address,uint256,uint256,bytes32[])")[..4].try_into().unwrap(),
        id("unoswapTo(address,address,uint256,uint256,bytes32[])")[..4].try_into().unwrap(),
        id("unoswapWithPermit(address,uint256,uint256,bytes32[],uint256,uint256,uint8,bytes32,bytes32)")[..4].try_into().unwrap(),
        id("unoswapToWithPermit(address,address,uint256,uint256,bytes32[],uint256,uint256,uint8,bytes32,bytes32)")[..4].try_into().unwrap(),
        id("uniswapV3Swap(uint256,uint256,uint256[])")[..4].try_into().unwrap(),
        id("uniswapV3SwapTo(address,uint256,uint256,uint256[])")[..4].try_into().unwrap(),
        id("uniswapV3SwapToWithPermit(address,uint256,uint256,uint256[],uint256,uint256,uint8,bytes32,bytes32)")[..4].try_into().unwrap(),
        id("clipperSwap(address,address,uint256,uint256,uint256,uint256)")[..4].try_into().unwrap(),
        // selector observed in production transactions
        [0x07, 0xed, 0x23, 0x79],
    ]
});

pub async fn analyze_oneinch_aggregation_router_v6(
    _rpc_client: Arc<dyn RpcProvider>,
    _rpc_endpoint: String,
    tx: TransactionData,
    _block: Option<u64>,
    outcome: TxLogs,
    router: RouterInfo,
) -> Result<AnalysisResult> {
    if tx.data.len() < 4 || !SWAP_SELECTORS.iter().any(|s| tx.data[..4] == s[..]) {
        return Err(anyhow::anyhow!("not aggregation router v6 swap"));
    }

    // identify called swap function
    let (swap_function, _) = detect_swap_function(&tx.data).unwrap_or((
        SwapFunction::AggregationRouterV6Swap,
        ethers::abi::AbiParser::default()
            .parse_function("aggregationSwap(bytes)")
            .unwrap(),
    ));

    use ethers::types::H256;
    use ethers::utils::keccak256;

    let transfer_sig: H256 =
        H256::from_slice(keccak256("Transfer(address,address,uint256)").as_slice());
    let mut src_token: Option<Address> = None;
    let mut dst_token: Option<Address> = None;

    for log in &outcome.logs {
        if log.topics.get(0) == Some(&transfer_sig) && log.topics.len() == 3 {
            let from = Address::from_slice(&log.topics[1].as_bytes()[12..]);
            let to = Address::from_slice(&log.topics[2].as_bytes()[12..]);
            if from == tx.from && src_token.is_none() {
                src_token = Some(log.address);
            }
            if to == tx.from && dst_token.is_none() {
                dst_token = Some(log.address);
            }
        }
    }

    let metrics = Metrics {
        swap_function,
        token_route: match (src_token, dst_token) {
            (Some(a), Some(b)) => vec![a, b],
            _ => Vec::new(),
        },
        slippage: 0.0,
        min_tokens_to_affect: U256::zero(),
        potential_profit: U256::zero(),
        router_address: router.address,
        router_name: None,
    };

    Ok(AnalysisResult {
        potential_victim: false,
        economically_viable: false,
        simulated_tx: None,
        metrics,
    })
}
