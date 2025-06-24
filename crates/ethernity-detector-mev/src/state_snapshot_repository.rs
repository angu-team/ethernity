use crate::state_impact_evaluator::StateSnapshot;
use crate::tx_aggregator::TxGroup;
use ethernity_core::error::{Error, Result};
use ethernity_core::traits::RpcProvider;
use ethereum_types::{Address, H256, U256};
use redb::{Database, TableDefinition};

const SNAPSHOT_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("snapshots");
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
    db: Database,
    history: Mutex<HashMap<Address, Vec<PersistedSnapshot>>>,
}

impl<P: RpcProvider> StateSnapshotRepository<P> {
    pub fn open(provider: P, path: impl AsRef<Path>) -> Result<Self> {
        let base = path.as_ref();
        let db_path = if base.is_dir() { base.join("db.redb") } else { base.to_path_buf() };
        let db = Database::create(db_path).map_err(|e| Error::Other(e.to_string()))?;
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
        let mut txn = self.db.begin_write().unwrap();
        {
            let mut table = txn.open_table(SNAPSHOT_TABLE).unwrap();
            let _ = table.insert(key.as_bytes(), bytes.as_slice());
        }
        txn.commit().unwrap();

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
                let mut tx2 = self.db.begin_write().unwrap();
                {
                    let mut table = tx2.open_table(SNAPSHOT_TABLE).unwrap();
                    let data = serde_json::to_vec(&curr_entry).unwrap();
                    let _ = table.insert(key.as_bytes(), data.as_slice());
                }
                tx2.commit().unwrap();
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
            let mut need_fetch = true;
            {
                let read_txn = self.db.begin_read().unwrap();
                if let Ok(table) = read_txn.open_table(SNAPSHOT_TABLE) {
                    if let Ok(Some(data)) = table.get(key.as_bytes()) {
                        if let Ok(saved) = serde_json::from_slice::<PersistedSnapshot>(data.value()) {
                            if saved.block_hash == block_hash {
                                need_fetch = false;
                            }
                        }
                    }
                }
            }
            if !need_fetch {
                continue;
            } else {
                let mut tx = self.db.begin_write().unwrap();
                {
                    let mut table = tx.open_table(SNAPSHOT_TABLE).unwrap();
                    let _ = table.remove(key.as_bytes());
                }
                tx.commit().unwrap();
            }
            let mut snap = self.fetch_v2_snapshot(target).await?;
            snap.state_lag_blocks = 1;
            self.store_snapshot(target, block_number, block_hash, profile, snap, origin);
        }
        Ok(())
    }

    pub fn get_state(&self, address: Address, block_number: u64, profile: SnapshotProfile) -> Option<StateSnapshot> {
        let key = Self::key(address, block_number, profile);
        let read_txn = self.db.begin_read().ok()?;
        let table = read_txn.open_table(SNAPSHOT_TABLE).ok()?;
        match table.get(key.as_bytes()) {
            Ok(Some(data)) => serde_json::from_slice::<PersistedSnapshot>(data.value()).ok().map(|e| e.snapshot),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ethernity_core::{traits::RpcProvider, error::Result, types::TransactionHash};
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[derive(Clone, Default)]
    struct DummyProvider {
        responses: Arc<Mutex<Vec<Vec<u8>>>>,
        call_count: Arc<Mutex<usize>>,
        hashes: Arc<Mutex<HashMap<u64, H256>>>,
    }

    impl DummyProvider {
        fn push_response(&self, r0: u128, r1: u128) {
            let mut buf = vec![0u8; 64];
            U256::from(r0).to_big_endian(&mut buf[0..32]);
            U256::from(r1).to_big_endian(&mut buf[32..64]);
            self.responses.lock().unwrap().push(buf);
        }

        fn push_raw(&self, data: Vec<u8>) {
            self.responses.lock().unwrap().push(data);
        }

        fn set_hash(&self, block: u64, hash: H256) {
            self.hashes.lock().unwrap().insert(block, hash);
        }

        fn calls(&self) -> usize {
            *self.call_count.lock().unwrap()
        }
    }

    #[async_trait]
    impl RpcProvider for DummyProvider {
        async fn get_transaction_trace(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
        async fn get_transaction_receipt(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
        async fn get_code(&self, _address: Address) -> Result<Vec<u8>> { Ok(vec![]) }
        async fn call(&self, _to: Address, _data: Vec<u8>) -> Result<Vec<u8>> {
            *self.call_count.lock().unwrap() += 1;
            Ok(self.responses.lock().unwrap().remove(0))
        }
        async fn get_block_number(&self) -> Result<u64> { Ok(0) }
        async fn get_block_hash(&self, block_number: u64) -> Result<H256> {
            Ok(*self.hashes.lock().unwrap().get(&block_number).unwrap_or(&H256::zero()))
        }
    }

    fn make_group(target: Address) -> HashMap<H256, TxGroup> {
        let gid = H256::repeat_byte(0x11);
        let group = TxGroup {
            group_key: gid,
            token_paths: vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)],
            targets: vec![target],
            txs: Vec::new(),
            block_number: None,
            direction_signature: "sig".into(),
            ordering_certainty_score: 1.0,
            reorderable: false,
            contaminated: false,
            window_start: 0,
        };
        let mut map = HashMap::new();
        map.insert(gid, group);
        map
    }

    #[tokio::test]
    async fn open_and_persist() {
        let dir = TempDir::new().unwrap();
        let provider = DummyProvider::default();
        provider.push_response(1000, 1000);
        provider.set_hash(1, H256::repeat_byte(0x01));
        let repo = StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap();
        let groups = make_group(Address::repeat_byte(0xaa));
        repo.snapshot_groups(&groups, 1, SnapshotProfile::Basic).await.unwrap();
        drop(repo);

        let repo2 = StateSnapshotRepository::open(provider, dir.path()).unwrap();
        let snap = repo2.get_state(Address::repeat_byte(0xaa), 1, SnapshotProfile::Basic).unwrap();
        assert_eq!(snap.reserve_in, 1000.0);
    }

    #[tokio::test]
    async fn store_multi_profile() {
        let dir = TempDir::new().unwrap();
        let provider = DummyProvider::default();
        provider.push_response(1000, 1000);
        provider.push_response(2000, 2000);
        provider.set_hash(1, H256::repeat_byte(0x01));
        let repo = StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap();
        let groups = make_group(Address::repeat_byte(0xaa));
        repo.snapshot_groups(&groups, 1, SnapshotProfile::Basic).await.unwrap();
        repo.snapshot_groups(&groups, 1, SnapshotProfile::Extended).await.unwrap();
        let a = repo.get_state(Address::repeat_byte(0xaa), 1, SnapshotProfile::Basic).unwrap();
        let b = repo.get_state(Address::repeat_byte(0xaa), 1, SnapshotProfile::Extended).unwrap();
        assert_eq!(a.reserve_in, 1000.0);
        assert_eq!(b.reserve_in, 2000.0);
    }

    #[test]
    fn deterministic_key() {
        let addr = Address::repeat_byte(0xaa);
        let k1 = StateSnapshotRepository::<DummyProvider>::key(addr, 1, SnapshotProfile::Basic);
        let k2 = StateSnapshotRepository::<DummyProvider>::key(addr, 1, SnapshotProfile::Basic);
        assert_eq!(k1, k2);
    }

    #[tokio::test]
    async fn serialization_cycle() {
        let dir = TempDir::new().unwrap();
        let provider = DummyProvider::default();
        provider.push_response(1234, 5678);
        provider.set_hash(1, H256::repeat_byte(0x02));
        let repo = StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap();
        let groups = make_group(Address::repeat_byte(0xaa));
        repo.snapshot_groups(&groups, 1, SnapshotProfile::Basic).await.unwrap();
        let original = repo.get_state(Address::repeat_byte(0xaa), 1, SnapshotProfile::Basic).unwrap();
        drop(repo);
        let repo2 = StateSnapshotRepository::open(provider, dir.path()).unwrap();
        let loaded = repo2.get_state(Address::repeat_byte(0xaa), 1, SnapshotProfile::Basic).unwrap();
        assert_eq!(original.reserve_in, loaded.reserve_in);
        assert_eq!(original.reserve_out, loaded.reserve_out);
    }

    #[tokio::test]
    async fn history_limit_and_volatility() {
        let dir = TempDir::new().unwrap();
        let provider = DummyProvider::default();
        provider.set_hash(1, H256::from_low_u64_be(1));
        provider.set_hash(2, H256::from_low_u64_be(2));
        provider.set_hash(3, H256::from_low_u64_be(3));
        provider.set_hash(4, H256::from_low_u64_be(4));
        provider.push_response(1000, 0);
        provider.push_response(1000, 0);
        provider.push_response(1100, 0);
        provider.push_response(1200, 0);
        let repo = StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap();
        let groups = make_group(Address::repeat_byte(0xaa));
        repo.snapshot_groups(&groups, 1, SnapshotProfile::Basic).await.unwrap();
        repo.snapshot_groups(&groups, 2, SnapshotProfile::Basic).await.unwrap();
        repo.snapshot_groups(&groups, 3, SnapshotProfile::Basic).await.unwrap();
        repo.snapshot_groups(&groups, 4, SnapshotProfile::Basic).await.unwrap();
        let hist = repo.history.lock();
        let list = hist.get(&Address::repeat_byte(0xaa)).unwrap();
        assert_eq!(list.len(), 3);
        assert!(list.iter().all(|e| e.block_number >= 2));
        let last = repo.get_state(Address::repeat_byte(0xaa), 4, SnapshotProfile::Basic).unwrap();
        assert!(last.volatility_flag);
    }

    #[tokio::test]
    async fn fork_invalidation_and_refetch() {
        let dir = TempDir::new().unwrap();
        let provider = DummyProvider::default();
        provider.set_hash(1, H256::from_low_u64_be(1));
        provider.push_response(1000, 0);
        provider.push_response(2000, 0);
        let repo = StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap();
        let groups = make_group(Address::repeat_byte(0xaa));
        repo.snapshot_groups(&groups, 1, SnapshotProfile::Basic).await.unwrap();
        let c1 = provider.calls();
        // same hash should not trigger fetch
        provider.set_hash(1, H256::from_low_u64_be(1));
        repo.snapshot_groups(&groups, 1, SnapshotProfile::Basic).await.unwrap();
        assert_eq!(provider.calls(), c1);
        // change hash -> refetch
        provider.set_hash(1, H256::from_low_u64_be(2));
        repo.snapshot_groups(&groups, 1, SnapshotProfile::Basic).await.unwrap();
        assert!(provider.calls() > c1);
        let snap = repo.get_state(Address::repeat_byte(0xaa), 1, SnapshotProfile::Basic).unwrap();
        assert_eq!(snap.reserve_in, 2000.0);
    }

    #[tokio::test]
    async fn fetch_v2_snapshot_valid_invalid() {
        let dir = TempDir::new().unwrap();
        let provider = DummyProvider::default();
        provider.push_response(1000, 1000);
        provider.push_raw(vec![1, 2, 3]);
        let repo = StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap();
        let ok = repo.fetch_v2_snapshot(Address::repeat_byte(0xaa)).await.unwrap();
        assert_eq!(ok.reserve_in, 1000.0);
        let err = repo.fetch_v2_snapshot(Address::repeat_byte(0xaa)).await.unwrap_err();
        match err {
            Error::DecodeError(_) => {},
            _ => panic!("unexpected error"),
        }
    }

    #[tokio::test]
    async fn snapshot_groups_dedup_origin() {
        let dir = TempDir::new().unwrap();
        let provider = DummyProvider::default();
        provider.push_response(1000, 0);
        provider.set_hash(1, H256::from_low_u64_be(1));
        let repo = StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap();

        let target = Address::repeat_byte(0xaa);
        let mut groups = make_group(target);
        let gid2 = H256::repeat_byte(0x22);
        let group2 = TxGroup { targets: vec![target], group_key: gid2, ..groups.values().next().unwrap().clone() };
        groups.insert(gid2, group2);

        repo.snapshot_groups(&groups, 1, SnapshotProfile::Basic).await.unwrap();
        assert_eq!(provider.calls(), 1);

        let key = StateSnapshotRepository::<DummyProvider>::key(target, 1, SnapshotProfile::Basic);
        let read_txn = repo.db.begin_read().unwrap();
        let table = read_txn.open_table(SNAPSHOT_TABLE).unwrap();
        let raw = table.get(key.as_bytes()).unwrap().unwrap();
        let entry: PersistedSnapshot = serde_json::from_slice(raw.value()).unwrap();
        assert_eq!(entry.group_origin.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_snapshot_deadlock_prevention() {
        use futures::future::join_all;
        use std::sync::Arc;
        use tokio::time::{timeout, Duration};

        let dir = TempDir::new().unwrap();
        let provider = DummyProvider::default();
        for _ in 0..20 { provider.push_response(1000, 0); }
        provider.set_hash(1, H256::repeat_byte(0x01));
        let repo = Arc::new(StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap());

        let mut handles = Vec::new();
        for i in 0u8..20 {
            let r = Arc::clone(&repo);
            let groups = make_group(Address::repeat_byte(i));
            handles.push(tokio::spawn(async move {
                r.snapshot_groups(&groups, 1, SnapshotProfile::Basic).await.unwrap();
            }));
        }

        timeout(Duration::from_secs(5), join_all(handles)).await.unwrap();

        for i in 0u8..20 {
            assert!(repo.get_state(Address::repeat_byte(i), 1, SnapshotProfile::Basic).is_some());
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_db_write_corruption_prevention() {
        use std::sync::Arc;
        use std::thread;

        let dir = TempDir::new().unwrap();

        let provider1 = DummyProvider::default();
        provider1.push_response(1000, 0);
        provider1.set_hash(1, H256::from_low_u64_be(1));
        let repo1 = Arc::new(StateSnapshotRepository::open(provider1.clone(), dir.path()).unwrap());

        let provider2 = DummyProvider::default();
        provider2.push_response(2000, 0);
        provider2.set_hash(1, H256::from_low_u64_be(1));

        let target = Address::repeat_byte(0xaa);
        let groups = make_group(target);
        let path = dir.path().to_path_buf();
        let groups_clone = groups.clone();
        let handle = thread::spawn(move || {
            match StateSnapshotRepository::open(provider2, &path) {
                Ok(repo) => {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        repo.snapshot_groups(&groups_clone, 1, SnapshotProfile::Basic).await.unwrap();
                    });
                }
                Err(_) => {}
            }
        });

        repo1.snapshot_groups(&groups, 1, SnapshotProfile::Basic).await.unwrap();
        handle.join().unwrap();

        drop(repo1);
        let repo_final = StateSnapshotRepository::open(DummyProvider::default(), dir.path()).unwrap();
        let snap = repo_final.get_state(target, 1, SnapshotProfile::Basic).unwrap();
        assert!(snap.reserve_in == 1000.0 || snap.reserve_in == 2000.0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_db_write_corruption_prevention() {
        use futures::future::join_all;
        use std::sync::Arc;
        use tokio::time::{timeout, Duration};

        let dir = TempDir::new().unwrap();
        let provider = DummyProvider::default();
        provider.push_response(1000, 0);
        provider.push_response(2000, 0);
        provider.set_hash(1, H256::repeat_byte(0x01));

        let repo = Arc::new(StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap());
        let target = Address::repeat_byte(0xaa);

        let g1 = make_group(target);
        let g2 = make_group(target);

        let handles = vec![
            {
                let r = Arc::clone(&repo);
                tokio::spawn(async move {
                    r.snapshot_groups(&g1, 1, SnapshotProfile::Basic).await.unwrap();
                })
            },
            {
                let r = Arc::clone(&repo);
                tokio::spawn(async move {
                    r.snapshot_groups(&g2, 1, SnapshotProfile::Basic).await.unwrap();
                })
            },
        ];

        timeout(Duration::from_secs(5), join_all(handles)).await.unwrap();

        drop(repo);
        let repo_check = StateSnapshotRepository::open(provider, dir.path()).unwrap();
        let key = StateSnapshotRepository::<DummyProvider>::key(target, 1, SnapshotProfile::Basic);
        let read_txn = repo_check.db.begin_read().unwrap();
        let table = read_txn.open_table(SNAPSHOT_TABLE).unwrap();
        let raw = table.get(key.as_bytes()).unwrap().unwrap();
        let entry: PersistedSnapshot = serde_json::from_slice(raw.value()).unwrap();
        assert!(entry.snapshot.reserve_in == 1000.0 || entry.snapshot.reserve_in == 2000.0);
    }
}
