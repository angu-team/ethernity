use crate::dex::{detect_swap_function, RouterInfo, SwapFunction};
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, Metrics, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethereum_types::{Address, U256};
use ethernity_core::traits::RpcProvider;
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
        _outcome: SimulationOutcome,
        _router: RouterInfo,
    ) -> Result<AnalysisResult> {
        let (func, f) = detect_swap_function(&tx.data).ok_or(anyhow!("unrecognized swap"))?;
        if func != SwapFunction::SwapV3ExactIn {
            return Err(anyhow!("unsupported swap"));
        }
        let tokens = f.decode_input(&tx.data[4..])?;
        let params = tokens
            .get(0)
            .and_then(|t| t.clone().into_tuple())
            .ok_or_else(|| anyhow!("invalid params"))?;
        let token_in = params
            .get(0)
            .and_then(|t| t.clone().into_address())
            .ok_or_else(|| anyhow!("tokenIn"))?;
        let token_out = params
            .get(1)
            .and_then(|t| t.clone().into_address())
            .ok_or_else(|| anyhow!("tokenOut"))?;
        let through1 = params
            .get(2)
            .and_then(|t| t.clone().into_address())
            .unwrap_or(Address::zero());
        let through2 = params
            .get(3)
            .and_then(|t| t.clone().into_address())
            .unwrap_or(Address::zero());

        let mut token_route = vec![token_in];
        if through1 != Address::zero() {
            token_route.push(through1);
        }
        if through2 != Address::zero() {
            token_route.push(through2);
        }
        token_route.push(token_out);

        let metrics = Metrics {
            swap_function: func,
            token_route,
            slippage: 0.0,
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
    }
}
