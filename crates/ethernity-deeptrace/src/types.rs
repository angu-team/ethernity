use ethereum_types::{Address, H256, U256};
use crate::trace::{CallTree, CallType};

/// Resultado da análise de uma transação
#[derive(Debug)]
pub struct TransactionAnalysis {
    pub tx_hash: H256,
    pub block_number: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas_used: U256,
    pub status: bool,
    pub call_tree: CallTree,
    pub token_transfers: Vec<TokenTransfer>,
    pub contract_creations: Vec<ContractCreation>,
    pub detected_patterns: Vec<DetectedPattern>,
    pub execution_path: Vec<ExecutionStep>,
}

/// Transferência de token
#[derive(Debug, Clone, PartialEq)]
pub struct TokenTransfer {
    pub token_type: TokenType,
    pub token_address: Address,
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    pub token_id: Option<U256>,
    pub call_index: usize,
}

/// Tipo de token
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Erc20,
    Erc721,
    Erc1155,
    Unknown,
}

/// Criação de contrato
#[derive(Debug, Clone)]
pub struct ContractCreation {
    pub creator: Address,
    pub contract_address: Address,
    pub init_code: Vec<u8>,
    pub contract_type: ContractType,
    pub call_index: usize,
}

/// Tipo de contrato
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractType {
    Erc20Token,
    Erc721Token,
    Erc1155Token,
    DexPool,
    LendingPool,
    Proxy,
    Factory,
    Unknown,
}

/// Padrão detectado
#[derive(Debug, Clone)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub addresses: Vec<Address>,
    pub data: serde_json::Value,
    pub description: String,
}

/// Tipo de padrão
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    Erc20Creation,
    Unknown,
}

/// Passo de execução
#[derive(Debug, Clone)]
pub struct ExecutionStep {
    pub depth: usize,
    pub call_type: CallType,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub input: Vec<u8>,
    pub output: Vec<u8>,
    pub gas_used: U256,
    pub error: Option<String>,
}
