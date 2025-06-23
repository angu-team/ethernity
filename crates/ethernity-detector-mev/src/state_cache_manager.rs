use crate::state_impact_evaluator::StateSnapshot;
use crate::tx_aggregator::TxGroup;
use ethernity_core::error::{Error, Result};
use ethernity_core::traits::RpcProvider;
use ethereum_types::{Address, H256, U256};
use parking_lot::Mutex;
use std::collections::HashMap;

/// Perfil de granularidade para snapshot de estado
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapshotProfile {
    Basic,
    Extended,
    Deep,
}

impl SnapshotProfile {
    fn as_str(&self) -> &'static str {
        match self {
            SnapshotProfile::Basic => "basic",
            SnapshotProfile::Extended => "extended",
            SnapshotProfile::Deep => "deep",
        }
    }
}

impl From<&str> for SnapshotProfile {
    fn from(value: &str) -> Self {
        match value {
            "extended" => SnapshotProfile::Extended,
            "deep" => SnapshotProfile::Deep,
            _ => SnapshotProfile::Basic,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedSnapshot {
    snapshot: StateSnapshot,
    block_number: u64,
    timestamp: u64,
    profile: SnapshotProfile,
    group_origin: Vec<H256>,
}

/// Gerenciador de cache de snapshots de estado on-chain
pub struct StateCacheManager<P> {
    provider: P,
    cache: Mutex<HashMap<(Address, u64, SnapshotProfile), CachedSnapshot>>, // chave (target, blockNumber, profile)
    history: Mutex<HashMap<Address, Vec<CachedSnapshot>>>,
}

impl<P> StateCacheManager<P> {
    /// Cria um novo gerenciador a partir de um provider RPC
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            cache: Mutex::new(HashMap::new()),
            history: Mutex::new(HashMap::new()),
        }
    }
}

impl<P: RpcProvider> StateCacheManager<P> {
    /// Realiza snapshot dos targets provenientes do TxAggregator
    pub async fn snapshot_groups(
        &self,
        groups: &HashMap<H256, TxGroup>,
        block_number: u64,
        profile: SnapshotProfile,
    ) -> Result<()> {
        // deduplicação target -> grupos
        let mut dedup: HashMap<Address, Vec<H256>> = HashMap::new();
        for (gid, g) in groups {
            for t in &g.targets {
                dedup.entry(*t).or_default().push(*gid);
            }
        }
        for (target, origin) in dedup {
            if self
                .cache
                .lock()
                .contains_key(&(target, block_number, profile))
            {
                continue;
            }
            let mut snap = self.fetch_v2_snapshot(target).await?;
            snap.state_lag_blocks = 1; // pending+1
            self.store_snapshot(target, block_number, profile, snap, origin);
        }
        Ok(())
    }

    fn store_snapshot(
        &self,
        target: Address,
        block_number: u64,
        profile: SnapshotProfile,
        snapshot: StateSnapshot,
        origin: Vec<H256>,
    ) {
        let entry = CachedSnapshot {
            snapshot: snapshot.clone(),
            block_number,
            timestamp: chrono::Utc::now().timestamp() as u64,
            profile,
            group_origin: origin,
        };
        let key = (target, block_number, profile);
        {
            let mut c = self.cache.lock();
            c.insert(key, entry.clone());
        }
        let mut h = self.history.lock();
        let hist = h.entry(target).or_default();
        hist.push(entry.clone());
        // mantém apenas os últimos 3 blocos
        while hist.len() > 3 {
            hist.remove(0);
        }
        // calcula volatilidade simples
        if hist.len() >= 2 {
            let prev = &hist[hist.len() - 2];
            let curr = &hist[hist.len() - 1];
            let delta = if prev.snapshot.reserve_in != 0.0 {
                ((curr.snapshot.reserve_in - prev.snapshot.reserve_in).abs()
                    / prev.snapshot.reserve_in)
                    * 100.0
            } else {
                0.0
            };
            if delta > 5.0 {
                // atualiza flag
                let mut c = self.cache.lock();
                if let Some(e) = c.get_mut(&key) {
                    e.snapshot.volatility_flag = true;
                }
            }
        }
    }

    async fn fetch_v2_snapshot(&self, target: Address) -> Result<StateSnapshot> {
        // selector getReserves()
        let data = vec![0x09, 0x02, 0xf1, 0xac];
        let out = self.provider.call(target, data).await?;
        if out.len() < 64 {
            return Err(Error::DecodeError("invalid getReserves response".into()));
        }
        let r0 = U256::from_big_endian(&out[0..32]);
        let r1 = U256::from_big_endian(&out[32..64]);
        let reserve0 = r0.low_u128() as f64;
        let reserve1 = r1.low_u128() as f64;
        Ok(StateSnapshot {
            reserve_in: reserve0,
            reserve_out: reserve1,
            sqrt_price_x96: None,
            liquidity: None,
            state_lag_blocks: 0,
            reorg_risk_level: "low".to_string(),
            volatility_flag: false,
        })
    }

    /// Recupera snapshot armazenado
    pub fn get_state(
        &self,
        target: Address,
        block_number: u64,
        profile: SnapshotProfile,
    ) -> Option<StateSnapshot> {
        let c = self.cache.lock();
        c.get(&(target, block_number, profile)).map(|e| e.snapshot.clone())
    }

    /// Processes [`TxGroup`] events, performs state snapshot and emits [`SnapshotEvent`].
    pub async fn process_stream(
        &self,
        mut rx: tokio::sync::mpsc::Receiver<TxGroup>,
        tx: tokio::sync::mpsc::Sender<crate::events::SnapshotEvent>,
        profile: SnapshotProfile,
    ) {
        while let Some(group) = rx.recv().await {
            let block = self.provider.get_block_number().await.unwrap_or(0);
            let mut map = HashMap::new();
            let mut gmap = HashMap::new();
            gmap.insert(group.group_key, group.clone());
            if self.snapshot_groups(&gmap, block, profile).await.is_ok() {
                for t in &group.targets {
                    if let Some(s) = self.get_state(*t, block, profile) {
                        map.insert(*t, s);
                    }
                }
            }
            let ev = crate::events::SnapshotEvent { group, snapshots: map };
            let _ = tx.send(ev).await;
        }
    }
}

