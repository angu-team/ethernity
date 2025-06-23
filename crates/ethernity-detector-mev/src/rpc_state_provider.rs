use async_trait::async_trait;
use ethernity_core::{error::{Error, Result}, traits::RpcProvider};
use ethereum_types::{Address, U256};
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;

use crate::traits::StateProvider;

/// Implementação padrão de [`StateProvider`] usando [`RpcProvider`].
pub struct RpcStateProvider<P> {
    primary: P,
    fallback: Option<P>,
    reserves_cache: Mutex<LruCache<Address, (U256, U256)>>,
    slot0_cache: Mutex<LruCache<Address, (U256, U256)>>,
}

impl<P: RpcProvider + Clone> RpcStateProvider<P> {
    /// Cria um novo provedor apenas com instancia primária.
    pub fn new(primary: P) -> Self {
        Self {
            primary,
            fallback: None,
            reserves_cache: Mutex::new(LruCache::new(NonZeroUsize::new(128).unwrap())),
            slot0_cache: Mutex::new(LruCache::new(NonZeroUsize::new(128).unwrap())),
        }
    }

    /// Define provedor de fallback.
    pub fn with_fallback(primary: P, fallback: P) -> Self {
        Self {
            primary,
            fallback: Some(fallback),
            reserves_cache: Mutex::new(LruCache::new(NonZeroUsize::new(128).unwrap())),
            slot0_cache: Mutex::new(LruCache::new(NonZeroUsize::new(128).unwrap())),
        }
    }

    async fn call_inner(&self, provider: &P, to: Address, data: &[u8]) -> Result<Vec<u8>> {
        provider.call(to, data.to_vec()).await
    }
}

#[async_trait]
impl<P> StateProvider for RpcStateProvider<P>
where
    P: RpcProvider + Clone + Send + Sync,
{
    async fn reserves(&self, address: Address) -> Result<(U256, U256)> {
        if let Some(v) = self.reserves_cache.lock().get(&address).cloned() {
            return Ok(v);
        }
        let data = vec![0x09, 0x02, 0xf1, 0xac];
        let out = match self.call_inner(&self.primary, address, &data).await {
            Ok(v) => v,
            Err(e) => {
                if let Some(ref fb) = self.fallback {
                    fb.call(address, data.clone()).await?
                } else {
                    return Err(e);
                }
            }
        };
        if out.len() < 64 {
            return Err(Error::DecodeError("invalid getReserves response".into()));
        }
        let v0 = U256::from_big_endian(&out[0..32]);
        let v1 = U256::from_big_endian(&out[32..64]);
        let tuple = (v0, v1);
        self.reserves_cache.lock().put(address, tuple.clone());
        Ok(tuple)
    }

    async fn slot0(&self, address: Address) -> Result<(U256, U256)> {
        if let Some(v) = self.slot0_cache.lock().get(&address).cloned() {
            return Ok(v);
        }
        let data = vec![0x38, 0x50, 0xc7, 0xbd]; // selector slot0()
        let out = match self.call_inner(&self.primary, address, &data).await {
            Ok(v) => v,
            Err(e) => {
                if let Some(ref fb) = self.fallback {
                    fb.call(address, data.clone()).await?
                } else {
                    return Err(e);
                }
            }
        };
        if out.len() < 32 {
            return Err(Error::DecodeError("invalid slot0 response".into()));
        }
        let v0 = U256::from_big_endian(&out[0..32]);
        let v1 = if out.len() >= 64 {
            U256::from_big_endian(&out[32..64])
        } else {
            U256::zero()
        };
        let tuple = (v0, v1);
        self.slot0_cache.lock().put(address, tuple.clone());
        Ok(tuple)
    }
}
