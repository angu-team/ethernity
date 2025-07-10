use crate::simulation::error::{Result, SimulationError};
use dashmap::DashMap;
use ethers::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use url::Url;

/// Sessão de simulação com uma instância Anvil
pub struct Session {
    pub provider: Provider<Http>,
    _anvil: ethers::utils::AnvilInstance,
    pub block: u64,
    pub last_used: Instant,
}

impl Session {
    fn normalize_rpc_url(endpoint: &str) -> String {
        match Url::parse(endpoint) {
            Ok(mut url) => {
                match url.scheme() {
                    "ws" => {
                        let _ = url.set_scheme("http");
                    }
                    "wss" => {
                        let _ = url.set_scheme("https");
                    }
                    _ => {}
                }
                url.to_string()
            }
            Err(_) => endpoint.to_string(),
        }
    }

    pub fn new(rpc: &str, block: u64) -> Result<Self> {
        let rpc = Self::normalize_rpc_url(rpc);
        let anvil = ethers::utils::Anvil::new()
            .fork(rpc)
            .fork_block_number(block)
            .spawn();
        let provider = Provider::<Http>::try_from(anvil.endpoint())
            .map_err(|e| SimulationError::ProviderCreation(e.to_string()))?
            .interval(Duration::from_millis(1));
        Ok(Session {
            provider,
            _anvil: anvil,
            block,
            last_used: Instant::now(),
        })
    }
}

/// Pool simples de sessões
pub struct SessionPool {
    ttl: Duration,
    sessions: DashMap<u64, Arc<Mutex<Session>>>, // keyed by block
}

impl SessionPool {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            sessions: DashMap::new(),
        }
    }

    /// Obtém ou cria sessão para o bloco indicado
    pub fn get_session(&self, rpc: &str, block: u64) -> Result<Arc<Mutex<Session>>> {
        // remove sessões expiradas
        let ttl = self.ttl;
        self.sessions
            .retain(|_, sess| sess.lock().last_used.elapsed() < ttl);
        if let Some(sess) = self.sessions.get(&block) {
            sess.lock().last_used = Instant::now();
            return Ok(sess.clone());
        }
        let session = Arc::new(Mutex::new(Session::new(rpc, block)?));
        self.sessions.insert(block, session.clone());
        Ok(session)
    }
}
