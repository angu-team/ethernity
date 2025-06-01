/*!
 * Ethernity Types
 * 
 * Tipos comuns usados em toda a workspace Ethernity
 */

use ethereum_types::{Address, H256, U256};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Alias para hash de transação
pub type TransactionHash = H256;

/// Tipo de evento blockchain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    Erc20Created,
    TokenSwap,
    LargeTransfer,
    Liquidation,
    RugPullWarning,
    MevActivity,
    FlashLoan,
    GovernanceEvent,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::Erc20Created => write!(f, "erc20_created"),
            EventType::TokenSwap => write!(f, "token_swap"),
            EventType::LargeTransfer => write!(f, "large_transfer"),
            EventType::Liquidation => write!(f, "liquidation"),
            EventType::RugPullWarning => write!(f, "rug_pull_warning"),
            EventType::MevActivity => write!(f, "mev_activity"),
            EventType::FlashLoan => write!(f, "flash_loan"),
            EventType::GovernanceEvent => write!(f, "governance_event"),
        }
    }
}

/// Informações sobre um token
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: Address,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub total_supply: Option<U256>,
}

/// Protocolo DEX
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DexProtocol {
    UniswapV2,
    UniswapV3,
    SushiSwap,
    Curve,
    Balancer,
    OneInch,
    Paraswap,
    Unknown(String),
}

/// Tipo de criação de contrato
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreationType {
    Create,
    Create2,
}

/// Padrão de contrato
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractPattern {
    Erc20Token,
    Proxy,
    Diamond,
    MinimalProxy,
    Factory,
    Multisig,
    Unknown,
}

/// Tipo de ataque
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    Reentrancy,
    FlashLoanAttack,
    PriceManipulation,
    GovernanceAttack,
    RugPull,
    Honeypot,
    GasBomb,
    FrontRunning,
    SandwichAttack,
}

/// Tipo de fluxo de fundos
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlowType {
    DirectTransfer,
    SwapOperation,
    LiquidityProvision,
    LiquidityRemoval,
    Arbitrage,
    FeeCollection,
}

/// Fonte de evento
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventSource {
    Mempool,
    Block,
    DeepTrace,
}

/// Severidade
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Status de transação
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionStatus {
    Success,
    Failure,
    Pending,
}

/// Identificador de usuário
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

/// Identificador de inscrição
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionId(pub String);

/// Identificador de evento
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub String);

/// Identificador de notificação
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NotificationId(pub String);

/// Identificador de conexão
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub String);
