use std::time::{Duration, Instant};

use async_trait::async_trait;
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::{transaction::eip2718::TypedTransaction, TransactionReceipt};
use ethers::utils::{Anvil, AnvilInstance};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    errors::{Result, SimulationError},
    logger::{log_error, log_warn},
    traits::{SimulationProvider, SimulationSession},
};

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
        let (closed, provider) = {
            let guard = self.lock().await;
            (guard.closed, guard.provider.clone())
        };
        if closed {
            log_warn("tentativa de uso de sessao encerrada").await;
            return Err(SimulationError::SessionClosed);
        }

        let pending = match provider.send_transaction(tx.clone(), None).await {
            Ok(p) => p,
            Err(e) => {
                log_error(&format!("falha ao enviar transacao: {}", e)).await;
                return Err(SimulationError::SendTransaction(e.to_string()));
            }
        };

        let receipt = match pending.await {
            Ok(opt) => match opt {
                Some(r) => r,
                None => {
                    log_error("falha ao aguardar transacao: sem recibo").await;
                    return Err(SimulationError::AwaitTransaction("sem recibo".into()));
                }
            },
            Err(e) => {
                log_error(&format!("falha ao aguardar transacao: {}", e)).await;
                return Err(SimulationError::AwaitTransaction(e.to_string()));
            }
        };
        Ok(receipt)
    }

    async fn close(&self) {
        let mut guard = self.lock().await;
        if guard.closed {
            log_warn("tentativa de encerrar sessao ja fechada").await;
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

    async fn create_session(
        &self,
        rpc_url: &str,
        block_number: Option<u64>,
        timeout: Duration,
    ) -> Result<Self::Session> {
        let mut anvil = Anvil::new()
            .fork(rpc_url)
            .args(&["--auto-impersonate".to_string()]);
        if let Some(block) = block_number {
            anvil = anvil.fork_block_number(block);
        }
        let anvil = anvil.spawn();

        let provider = match Provider::<Http>::try_from(anvil.endpoint()) {
            Ok(p) => p.interval(Duration::from_millis(1)),
            Err(e) => {
                log_error(&format!("falha ao criar provider do anvil: {}", e)).await;
                return Err(SimulationError::ProviderCreation(e.to_string()));
            }
        };

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
