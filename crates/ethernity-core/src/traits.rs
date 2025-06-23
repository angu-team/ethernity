/*!
 * Ethernity Traits
 * 
 * Traits comuns usados em toda a workspace Ethernity
 */

use async_trait::async_trait;
use crate::error::Result;
use crate::types::{EventType, TransactionHash};
use ethereum_types::Address;

/// Trait para provedores RPC
#[async_trait]
pub trait RpcProvider: Send + Sync {
    /// Obtém o trace de uma transação
    async fn get_transaction_trace(&self, tx_hash: TransactionHash) -> Result<Vec<u8>>;
    
    /// Obtém o recibo de uma transação
    async fn get_transaction_receipt(&self, tx_hash: TransactionHash) -> Result<Vec<u8>>;
    
    /// Obtém o código de um contrato
    async fn get_code(&self, address: Address) -> Result<Vec<u8>>;
    
    /// Chama um método de contrato
    async fn call(&self, to: Address, data: Vec<u8>) -> Result<Vec<u8>>;

    /// Obtém o número do bloco atual
    async fn get_block_number(&self) -> Result<u64>;

    /// Obtém o hash de um bloco
    async fn get_block_hash(&self, block_number: u64) -> Result<ethereum_types::H256>;
}

/// Trait para detectores de eventos
#[async_trait]
pub trait EventDetector: Send + Sync {
    /// Tipo de evento detectado
    fn event_type(&self) -> EventType;
    
    /// Detecta eventos em uma transação
    async fn detect_events(&self, tx_hash: TransactionHash) -> Result<Vec<u8>>;
    
    /// Verifica se uma transação requer análise profunda
    fn requires_deep_trace(&self, tx_hash: TransactionHash) -> bool;
}

/// Trait para notificadores de eventos
#[async_trait]
pub trait EventNotifier: Send + Sync {
    /// Envia uma notificação de evento
    async fn notify(&self, event_data: Vec<u8>) -> Result<()>;
    
    /// Verifica se o notificador está disponível
    async fn is_available(&self) -> bool;
}
