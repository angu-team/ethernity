use crate::dex::RouterInfo;
use crate::simulation::SimulationOutcome;
use crate::types::{AnalysisResult, TransactionData};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethernity_core::traits::RpcProvider;
use ethers::types::H256;
use std::str::FromStr;
use std::sync::Arc;

/// Detector para interações de swap na arquitetura Uniswap V4.
/// Atualmente realiza apenas a identificação do swap através do evento
/// `Swap(bytes32,address,int128,int128,uint160,uint128,int24,uint24)`.
/// Caso identificado, o detector retorna um erro indicando que a
/// implementação detalhada ainda não está disponível.
pub struct UniswapV4Detector;

const UNISWAP_V4_SWAP_TOPIC: &str = "0xfbc3feb9544dba19141913965b8f867f5d0d220b898fc1b39e7d7111686a8f51";

#[async_trait]
impl crate::detectors::VictimDetector for UniswapV4Detector {
    fn supports(&self, _router: &RouterInfo) -> bool {
        true
    }

    async fn analyze(
        &self,
        _rpc_client: Arc<dyn RpcProvider>,
        _rpc_endpoint: String,
        _tx: TransactionData,
        _block: Option<u64>,
        outcome: SimulationOutcome,
        _router: RouterInfo,
    ) -> Result<AnalysisResult> {
        let topic = H256::from_str(UNISWAP_V4_SWAP_TOPIC).expect("valid topic hex");
        if outcome.logs.iter().any(|log| log.topics.get(0) == Some(&topic)) {
            Err(anyhow!("uniswap v4 detector not implemented"))
        } else {
            Err(anyhow!("no uniswap v4 swap event"))
        }
    }
}
