use crate::dex::SwapFunction;
use ethereum_types::{Address, H256, U256};
use serde::{Deserialize, Serialize};

/// Dados básicos de uma transação Ethereum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub from: Address,
    pub to: Address,
    pub data: Vec<u8>,
    pub value: U256,
    pub gas: u64,
    pub gas_price: U256,
    pub nonce: U256,
}

/// Métricas extraídas durante a simulação
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub swap_function: SwapFunction,
    pub token_route: Vec<Address>,
    pub slippage: f64,
    pub min_tokens_to_affect: U256,
    pub potential_profit: U256,
    pub router_address: Address,
    pub router_name: Option<String>,
}

/// Resultado final da análise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub potential_victim: bool,
    pub metrics: Metrics,
    pub economically_viable: bool,
    pub simulated_tx: Option<H256>,
}
