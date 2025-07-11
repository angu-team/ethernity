use std::time::{Duration, Instant};

use ethers::utils::{Anvil, AnvilInstance};
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{TransactionReceipt, transaction::eip2718::TypedTransaction};
use tokio::sync::Mutex;
use async_trait::async_trait;
use uuid::Uuid;

use crate::{errors::{Result, SimulationError}, traits::{SimulationProvider, SimulationSession}};

/// Sessão de simulação utilizando o Anvil
pub struct AnvilSession {
    pub id: Uuid,
    provider: Provider<Http>,
    _anvil: AnvilInstance,
    created: Instant,
    timeout: Duration,
}

impl AnvilSession {
    fn expired(&self) -> bool {
        self.created.elapsed() > self.timeout
    }
}

#[async_trait]
impl SimulationSession for Mutex<AnvilSession> {
    async fn send_transaction(&self, tx: &TypedTransaction) -> Result<TransactionReceipt> {
        let provider = {
            let mut guard = self.lock().await;
            guard.created = Instant::now();
            guard.provider.clone()
        };
        let pending = provider
            .send_transaction(tx.clone(), None)
            .await
            .map_err(|e| SimulationError::SendTransaction(e.to_string()))?;
        let receipt = pending
            .await
            .map_err(|e| SimulationError::AwaitTransaction(e.to_string()))?
            .ok_or_else(|| SimulationError::AwaitTransaction("sem recibo".into()))?;
        Ok(receipt)
    }

    async fn close(&self) {
        let _ = self.lock().await;
    }
}

pub struct AnvilProvider;

#[async_trait]
impl SimulationProvider for AnvilProvider {
    type Session = Mutex<AnvilSession>;

    async fn create_session(&self, rpc_url: &str, block_number: u64, timeout: Duration) -> Result<Self::Session> {
        let anvil = Anvil::new()
            .fork(rpc_url)
            .fork_block_number(block_number)
            .spawn();

        let provider = Provider::<Http>::try_from(anvil.endpoint())
            .map_err(|e| SimulationError::ProviderCreation(e.to_string()))?
            .interval(Duration::from_millis(1));

        Ok(Mutex::new(AnvilSession {
            id: Uuid::new_v4(),
            provider,
            _anvil: anvil,
            created: Instant::now(),
            timeout,
        }))
    }
}
