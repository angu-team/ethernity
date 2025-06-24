use ethernity_detector_mev::{mempool_supervisor::{MempoolSupervisor, OperationalMode}, AnnotatedTx};
use ethernity_core::{traits::RpcProvider, error::Result, types::TransactionHash};
use ethernity_detector_mev::events::{SupervisorEvent, BlockMetadata};
use ethereum_types::{Address, H256, U256};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
struct DummyProvider { block: Arc<Mutex<u64>> }

#[async_trait]
impl RpcProvider for DummyProvider {
    async fn get_transaction_trace(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_transaction_receipt(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_code(&self, _address: Address) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn call(&self, _to: Address, _data: Vec<u8>) -> Result<Vec<u8>> {
        let mut out = vec![0u8; 96];
        U256::from(1000u64).to_big_endian(&mut out[0..32]);
        U256::from(1000u64).to_big_endian(&mut out[32..64]);
        Ok(out)
    }
    async fn get_block_number(&self) -> Result<u64> { Ok(*self.block.lock().unwrap()) }
    async fn get_block_hash(&self, _block_number: u64) -> Result<H256> { Ok(H256::zero()) }
}

fn sample_tx(idx: u8, gas: f64, ts: u64) -> AnnotatedTx {
    AnnotatedTx {
        tx_hash: H256::repeat_byte(idx),
        token_paths: vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)],
        targets: vec![Address::repeat_byte(0xaa + idx)],
        tags: vec!["swap-v2".to_string()],
        first_seen: ts,
        gas_price: gas,
        max_priority_fee_per_gas: None,
        confidence: 1.0,
    }
}


#[tokio::test]
async fn mode_adaptation() {
    let _ = std::fs::remove_dir_all("snapshot_db");
    let provider = DummyProvider::default();
    let mut supervisor = MempoolSupervisor::new(provider.clone(), 1, Duration::from_secs(1), 10);
    supervisor.last_tick = Instant::now() - Duration::from_millis(100);
    for i in 0..10 { supervisor.ingest_tx(sample_tx(i, 10.0, i as u64)); }
    *provider.block.lock().unwrap() = 0;
    supervisor.tick().await.unwrap();
    assert_eq!(supervisor.operational_mode, OperationalMode::Burst);
    supervisor.last_tick = Instant::now() - Duration::from_secs(2);
    *provider.block.lock().unwrap() = 0;
    supervisor.tick().await.unwrap();
    assert_eq!(supervisor.operational_mode, OperationalMode::Normal);
}

#[test]
fn adaptive_ttl_modes() {
    let provider = DummyProvider::default();
    let mut supervisor = MempoolSupervisor::new(provider, 1, Duration::from_secs(1), 10);
    let high = sample_tx(1, 200.0, 0);
    let low = sample_tx(2, 10.0, 0);
    supervisor.operational_mode = OperationalMode::Normal;
    assert_eq!(supervisor.adaptive_ttl(&low), Duration::from_secs(5));
    assert_eq!(supervisor.adaptive_ttl(&high), Duration::from_secs(3));
    supervisor.operational_mode = OperationalMode::Burst;
    assert_eq!(supervisor.adaptive_ttl(&low), Duration::from_secs(3));
    supervisor.operational_mode = OperationalMode::Recovery;
    assert_eq!(supervisor.adaptive_ttl(&low), Duration::from_secs(7));
}

#[tokio::test]
async fn buffer_expiration() {
    let _ = std::fs::remove_dir_all("snapshot_db");
    let provider = DummyProvider::default();
    let mut sup = MempoolSupervisor::new(provider.clone(), 1, Duration::from_secs(1), 10);
    let tx = sample_tx(1, 10.0, 0);
    let hash = tx.tx_hash;
    sup.ingest_tx(tx);
    if let Some(mut entry) = sup.buffer.get_mut(&hash) { entry.expires_at = Instant::now() - Duration::from_secs(1); }
    sup.last_tick = Instant::now() - Duration::from_secs(1);
    *provider.block.lock().unwrap() = 0;
    sup.tick().await.unwrap();
    assert!(sup.buffer.is_empty());
}

#[tokio::test]
async fn window_increment_on_block() {
    let _ = std::fs::remove_dir_all("snapshot_db");
    let provider = DummyProvider::default();
    let mut sup = MempoolSupervisor::new(provider.clone(), 1, Duration::from_secs(1), 10);
    sup.ingest_tx(sample_tx(1, 10.0, 0));
    sup.last_tick = Instant::now() - Duration::from_secs(1);
    *provider.block.lock().unwrap() = 1;
    sup.tick().await.unwrap();
    assert_eq!(sup.window_id, 1);
}

#[tokio::test]
async fn jitter_and_alignment() {
    let _ = std::fs::remove_dir_all("snapshot_db");
    let provider = DummyProvider::default();
    let mut sup = MempoolSupervisor::new(provider.clone(), 1, Duration::from_secs(1), 10);
    let tx1 = sample_tx(1, 10.0, 1);
    let tx2 = sample_tx(2, 10.0, 3);
    sup.ingest_tx(tx1.clone());
    sup.ingest_tx(tx2.clone());
    sup.last_tick = Instant::now() - Duration::from_secs(1);
    *provider.block.lock().unwrap() = 0;
    sup.tick().await.unwrap();
    *provider.block.lock().unwrap() = 1;
    let groups = sup.tick().await.unwrap();
    assert_eq!(groups.len(), 1);
    let meta = &groups[0].metadata;
    assert!(meta.timestamp_jitter_score > 0.0);
    assert_eq!(meta.window_id, 1);
    assert!(meta.state_alignment_score > 0.5);
}

#[tokio::test]
async fn confidence_penalty_overlap() {
    let _ = std::fs::remove_dir_all("snapshot_db");
    let provider = DummyProvider::default();
    let mut sup = MempoolSupervisor::new(provider.clone(), 1, Duration::from_secs(1), 10);
    let tx = sample_tx(1, 10.0, 1);
    sup.ingest_tx(tx.clone());
    // new block before tick
    let (tx_chan, mut rx) = tokio::sync::mpsc::channel(10);
    sup.handle_event(SupervisorEvent::BlockAdvanced(BlockMetadata { number: 1 }), &tx_chan).await.unwrap();
    *provider.block.lock().unwrap() = 1;
    sup.last_tick = Instant::now() - Duration::from_secs(1);
    sup.tick().await.unwrap();
    *provider.block.lock().unwrap() = 2;
    sup.handle_event(SupervisorEvent::BlockAdvanced(BlockMetadata { number: 2 }), &tx_chan).await.unwrap();
    drop(tx_chan);
    let g = rx.recv().await.unwrap();
    assert!((g.group.txs[0].confidence - 0.9).abs() < 1e-6);
}
