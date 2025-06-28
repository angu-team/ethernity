use anyhow::Result;
use dashmap::DashMap;
use ethers::prelude::*;
use std::time::{Duration, Instant};

pub struct CachedRpc {
    provider: Provider<Http>,
    cache: DashMap<Vec<u8>, (Bytes, Instant)>,
    ttl: Duration,
}

impl CachedRpc {
    pub fn new(endpoint: String, ttl: Duration) -> Result<Self> {
        let provider = Provider::<Http>::try_from(endpoint)?.interval(Duration::from_millis(100));
        Ok(Self { provider, cache: DashMap::new(), ttl })
    }

    pub async fn call(&self, req: &ethers::types::TransactionRequest) -> Result<Bytes> {
        let key = serde_json::to_vec(req)?;
        if let Some(entry) = self.cache.get(&key) {
            let (val, t) = entry.value();
            if t.elapsed() < self.ttl {
                return Ok(val.clone());
            }
        }
        let out = self.provider.call(&req.clone().into(), None).await?;
        self.cache.insert(key, (out.clone(), Instant::now()));
        Ok(out)
    }

    pub fn provider(&self) -> &Provider<Http> {
        &self.provider
    }
}

