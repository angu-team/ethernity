use async_trait::async_trait;
use ethers::types::{TransactionReceipt, transaction::eip2718::TypedTransaction};
use std::time::Duration;

use crate::errors::Result;

#[async_trait]
pub trait SimulationSession: Send + Sync {
    /// Envia uma transação para a sessão simulada
    async fn send_transaction(&self, tx: &TypedTransaction) -> Result<TransactionReceipt>;

    /// Encerra a sessão
    async fn close(&self);
}

#[async_trait]
pub trait SimulationProvider: Send + Sync {
    type Session: SimulationSession;

    /// Cria uma nova sessão de simulação
    async fn create_session(&self, rpc_url: &str, block_number: u64, timeout: Duration) -> Result<Self::Session>;
}
