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
    anvil: Option<AnvilInstance>,
    created: Instant,
    timeout: Duration,
    closed: bool,
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
            let guard = self.lock().await;
            if guard.closed {
                return Err(SimulationError::SessionClosed);
            }
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
        let mut guard = self.lock().await;
        if guard.closed {
            return;
        }
        guard.closed = true;
        if let Some(anvil) = guard.anvil.take() {
            drop(anvil);
        }
    }
}

pub struct AnvilProvider;

#[async_trait]
impl SimulationProvider for AnvilProvider {
    type Session = Mutex<AnvilSession>;

    async fn create_session(&self, rpc_url: &str, block_number: Option<u64>, timeout: Duration) -> Result<Self::Session> {
        let mut anvil = Anvil::new()
            .fork(rpc_url)
            .args(&["--auto-impersonate".to_string()]);
        if let Some(block) = block_number {
            anvil = anvil.fork_block_number(block);
        }
        let anvil = anvil.spawn();

        let provider = Provider::<Http>::try_from(anvil.endpoint())
            .map_err(|e| SimulationError::ProviderCreation(e.to_string()))?
            .interval(Duration::from_millis(1));

        Ok(Mutex::new(AnvilSession {
            id: Uuid::new_v4(),
            provider,
            anvil: Some(anvil),
            created: Instant::now(),
            timeout,
            closed: false,
        }))
    }
}
