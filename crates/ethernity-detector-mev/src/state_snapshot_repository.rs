use crate::state_impact_evaluator::StateSnapshot;
use crate::tx_aggregator::TxGroup;
use ethernity_core::error::{Error, Result};
use ethernity_core::traits::RpcProvider;
use ethereum_types::{Address, H256, U256};
use rocksdb::{DB, Options};
use parking_lot::Mutex;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedSnapshot {
    snapshot: StateSnapshot,
    block_number: u64,
    block_hash: H256,
    timestamp: u64,
    profile: SnapshotProfile,
    group_origin: Vec<H256>,
}

pub struct StateSnapshotRepository<P> {
    provider: P,
    db: DB,
    history: Mutex<HashMap<Address, Vec<PersistedSnapshot>>>,
}

impl<P: RpcProvider> StateSnapshotRepository<P> {
    pub fn open(provider: P, path: impl AsRef<Path>) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).map_err(|e| Error::Other(e.to_string()))?;
        Ok(Self { provider, db, history: Mutex::new(HashMap::new()) })
    }

    fn key(address: Address, block_number: u64, profile: SnapshotProfile) -> String {
        format!("0x{:x}:{}:{}", address, block_number, profile.as_str())
    }

    async fn fetch_v2_snapshot(&self, target: Address) -> Result<StateSnapshot> {
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

    fn store_snapshot(&self, address: Address, block_number: u64, block_hash: H256, profile: SnapshotProfile, snapshot: StateSnapshot, origin: Vec<H256>) {
        let entry = PersistedSnapshot {
            snapshot: snapshot.clone(),
            block_number,
            block_hash,
            timestamp: chrono::Utc::now().timestamp() as u64,
            profile,
            group_origin: origin,
        };
        let key = Self::key(address, block_number, profile);
        let bytes = serde_json::to_vec(&entry).unwrap();
        let _ = self.db.put(key.as_bytes(), bytes);

        let mut h = self.history.lock();
        let hist = h.entry(address).or_default();
        hist.push(entry.clone());
        while hist.len() > 3 { hist.remove(0); }
        if hist.len() >= 2 {
            let prev = &hist[hist.len()-2];
            let curr = &hist[hist.len()-1];
            let delta = if prev.snapshot.reserve_in != 0.0 {
                ((curr.snapshot.reserve_in - prev.snapshot.reserve_in).abs() / prev.snapshot.reserve_in) * 100.0
            } else { 0.0 };
            if delta > 5.0 {
                let mut entry_mut = curr.snapshot.clone();
                entry_mut.volatility_flag = true;
                let mut curr_entry = entry.clone();
                curr_entry.snapshot = entry_mut.clone();
                let _ = self.db.put(key.as_bytes(), serde_json::to_vec(&curr_entry).unwrap());
            }
        }
    }

    pub async fn snapshot_groups(&self, groups: &HashMap<H256, TxGroup>, block_number: u64, profile: SnapshotProfile) -> Result<()> {
        let block_hash = self.provider.get_block_hash(block_number).await?;

        let mut dedup: HashMap<Address, Vec<H256>> = HashMap::new();
        for (gid, g) in groups {
            for t in &g.targets {
                dedup.entry(*t).or_default().push(*gid);
            }
        }

        for (target, origin) in dedup {
            let key = Self::key(target, block_number, profile);
            if let Ok(Some(data)) = self.db.get(key.as_bytes()) {
                if let Ok(saved) = serde_json::from_slice::<PersistedSnapshot>(&data) {
                    if saved.block_hash == block_hash {
                        continue;
                    } else {
                        let _ = self.db.delete(key.as_bytes());
                    }
                }
            }
            let mut snap = self.fetch_v2_snapshot(target).await?;
            snap.state_lag_blocks = 1;
            self.store_snapshot(target, block_number, block_hash, profile, snap, origin);
        }
        Ok(())
    }

    pub fn get_state(&self, address: Address, block_number: u64, profile: SnapshotProfile) -> Option<StateSnapshot> {
        let key = Self::key(address, block_number, profile);
        match self.db.get(key.as_bytes()) {
            Ok(Some(data)) => serde_json::from_slice::<PersistedSnapshot>(&data).ok().map(|e| e.snapshot),
            _ => None,
        }
    }
}
