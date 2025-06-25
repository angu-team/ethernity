use ethernity_detector_mev::{StateSnapshotRepository, SnapshotProfile, TxGroup};
use ethernity_core::{traits::RpcProvider, error::Result, types::TransactionHash};
use ethereum_types::{Address, H256, U256};
use async_trait::async_trait;
use tempfile::TempDir;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use redb::{Database, TableDefinition};

#[derive(Clone, Default)]
struct ChaosProvider {
    block: Arc<Mutex<u64>>,
    hashes: Arc<Mutex<HashMap<u64, H256>>>,
    responses: Arc<Mutex<Vec<Vec<u8>>>>,
    call_count: Arc<Mutex<usize>>,
}

impl ChaosProvider {
    fn push_response(&self, r0: u128, r1: u128) {
        let mut buf = vec![0u8; 64];
        U256::from(r0).to_big_endian(&mut buf[0..32]);
        U256::from(r1).to_big_endian(&mut buf[32..64]);
        self.responses.lock().unwrap().push(buf);
    }
    fn set_hash(&self, block: u64, hash: H256) { self.hashes.lock().unwrap().insert(block, hash); }
    fn set_block(&self, block: u64) { *self.block.lock().unwrap() = block; }
    fn calls(&self) -> usize { *self.call_count.lock().unwrap() }
}

#[async_trait]
impl RpcProvider for ChaosProvider {
    async fn get_transaction_trace(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_transaction_receipt(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_code(&self, _address: Address) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn call(&self, _to: Address, _data: Vec<u8>) -> Result<Vec<u8>> {
        *self.call_count.lock().unwrap() += 1;
        Ok(self.responses.lock().unwrap().remove(0))
    }
    async fn get_block_number(&self) -> Result<u64> { Ok(*self.block.lock().unwrap()) }
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

const SNAPSHOT_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("snapshots");

#[tokio::test]
async fn deep_reorg_and_db_corruption() {
    let dir = TempDir::new().unwrap();
    let provider = ChaosProvider::default();
    provider.set_block(0);
    let target = Address::repeat_byte(0xaa);
    let groups = make_group(target);
    let repo = StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap();

    for i in 1..=5u64 {
        provider.set_hash(i, H256::from_low_u64_be(i));
        provider.push_response(1000 + i as u128, 0);
        repo.snapshot_groups(&groups, i, SnapshotProfile::Basic).await.unwrap();
    }
    let initial_calls = provider.calls();
    drop(repo);

    // corrupt entries for blocks 2 and 4
    let db_path = dir.path().join("db.redb");
    let db = Database::open(db_path).unwrap();
    let tx = db.begin_write().unwrap();
    {
        let mut table = tx.open_table(SNAPSHOT_TABLE).unwrap();
        let k2 = format!("0x{:x}:2:basic", target);
        let k4 = format!("0x{:x}:4:basic", target);
        table.insert(k2.as_bytes(), &b"bad"[..]).unwrap();
        table.insert(k4.as_bytes(), &b"bad"[..]).unwrap();
    }
    tx.commit().unwrap();
    drop(db);

    let repo = StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap();
    for i in 1..=5u64 { provider.set_hash(i, H256::from_low_u64_be(100 + i)); }
    provider.push_response(2001, 0);
    repo.snapshot_groups(&groups, 5, SnapshotProfile::Basic).await.unwrap();
    let snap = repo.get_state(target, 5, SnapshotProfile::Basic).unwrap();
    assert_eq!(snap.reserve_in, 2001.0);
    assert!(provider.calls() > initial_calls);
    assert!(repo.get_state(target, 2, SnapshotProfile::Basic).is_none());
}
