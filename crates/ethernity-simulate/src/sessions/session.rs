use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::{logger::log_warn, traits::SimulationSession};

/// Estrutura interna de controle de sessão
pub struct SessionEntry<S: SimulationSession> {
    pub id: Uuid,
    pub session: Arc<S>,
    pub created: Instant,
    pub timeout: Duration,
}

impl<S: SimulationSession> SessionEntry<S> {
    pub fn expired(&self) -> bool {
        self.created.elapsed() > self.timeout
    }
}

/// Gerenciador de sessões
pub struct SessionManager<S: SimulationSession> {
    sessions: DashMap<Uuid, SessionEntry<S>>,
}

impl<S: SimulationSession> SessionManager<S> {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub fn insert(&self, session: Arc<S>, timeout: Duration) -> Uuid {
        let id = Uuid::new_v4();
        self.sessions.insert(
            id,
            SessionEntry {
                id,
                session,
                created: Instant::now(),
                timeout,
            },
        );
        id
    }

    pub fn get(&self, id: &Uuid) -> Option<Arc<S>> {
        self.cleanup();
        self.sessions.get(id).map(|e| e.session.clone())
    }

    pub fn remove(&self, id: &Uuid) {
        self.sessions.remove(id);
    }

    fn cleanup(&self) {
        let before = self.sessions.len();
        self.sessions
            .retain(|_, v| v.created.elapsed() <= v.timeout);
        let removed = before.saturating_sub(self.sessions.len());
        if removed > 0 {
            let msg = format!("{} sessoes expiradas removidas", removed);
            tokio::spawn(async move {
                log_warn(&msg).await;
            });
        }
    }
}
