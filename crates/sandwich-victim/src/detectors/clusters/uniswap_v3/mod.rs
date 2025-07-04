use crate::dex::RouterInfo;
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, Metrics, TransactionData};
use crate::core::metrics::U256Ext;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethernity_core::traits::RpcProvider;
use ethers::types::H256;
use ethers::utils::keccak256;
use ethereum_types::{Address, U256};
use std::sync::Arc;

/// Detector para funções do Uniswap V3 Router.
pub struct UniswapV3Detector;

#[async_trait]
impl crate::detectors::VictimDetector for UniswapV3Detector {
    fn supports(&self, router: &RouterInfo) -> bool {
        router.factory.is_none()
    }

    async fn analyze(
        &self,
        _rpc_client: Arc<dyn RpcProvider>,
        _rpc_endpoint: String,
        tx: TransactionData,
        _block: Option<u64>,
        outcome: SimulationOutcome,
        router: RouterInfo,
    ) -> Result<AnalysisResult> {
        const SELECTOR: [u8; 4] = [0x18, 0x61, 0xa3, 0xd8];
        if tx.data.len() < 4 || tx.data[..4] != SELECTOR {
            return Err(anyhow!("unsupported uniswap v3 swap"));
        }
        let data = &tx.data[4..];
        if data.len() < 32 * 10 {
            return Err(anyhow!("invalid calldata"));
        }
        let mut iter = data.chunks_exact(32);
        let token_in = Address::from_slice(&iter.next().unwrap()[12..]);
        let token_out = Address::from_slice(&iter.next().unwrap()[12..]);
        let through1 = Address::from_slice(&iter.next().unwrap()[12..]);
        let through2 = Address::from_slice(&iter.next().unwrap()[12..]);
        let _fee = U256::from_big_endian(iter.next().unwrap());
        let recipient = Address::from_slice(&iter.next().unwrap()[12..]);
        let _deadline = U256::from_big_endian(iter.next().unwrap());
        let _amount_in = U256::from_big_endian(iter.next().unwrap());
        let amount_out_min = U256::from_big_endian(iter.next().unwrap());
        let _sqrt_price_limit = U256::from_big_endian(iter.next().unwrap());
        let kind = crate::dex::SwapFunction::SwapV3ExactIn;

        let mut path = vec![token_in];
        if through1 != Address::zero() {
            path.push(through1);
        }
        if through2 != Address::zero() {
            path.push(through2);
        }
        path.push(token_out);

        let transfer_sig: H256 =
            H256::from_slice(keccak256("Transfer(address,address,uint256)").as_slice());
        let mut actual_out = U256::zero();
        for log in &outcome.logs {
            if log.topics.get(0) == Some(&transfer_sig) && log.topics.len() >= 3 {
                let to_addr = Address::from_slice(&log.topics[2].as_bytes()[12..]);
                if to_addr == recipient {
                    actual_out = U256::from_big_endian(&log.data.0);
                }
            }
        }
        let slippage = if actual_out < amount_out_min && !amount_out_min.is_zero() {
            (amount_out_min - actual_out).to_f64_lossy() / amount_out_min.to_f64_lossy()
        } else {
            0.0
        };

        let router_name = router
            .name
            .clone()
            .unwrap_or_else(|| format!("{:#x}", router.address));
        let metrics = Metrics {
            swap_function: kind,
            token_route: path,
            slippage,
            min_tokens_to_affect: U256::zero(),
            potential_profit: U256::zero(),
            router_address: router.address,
            router_name: Some(router_name),
        };

        Ok(AnalysisResult {
            potential_victim: slippage > 0.0,
            economically_viable: false,
            simulated_tx: None,
            metrics,
        })
    }
}
