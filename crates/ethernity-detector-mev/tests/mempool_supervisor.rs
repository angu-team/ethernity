use ethernity_detector_mev::{MempoolSupervisor, AnnotatedTx};
use ethernity_core::{traits::RpcProvider, error::Result, types::TransactionHash};
use ethereum_types::{Address, H256, U256};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
async fn window_increment_on_block() {
    let _ = std::fs::remove_dir_all("snapshot_db");
    let provider = DummyProvider::default();
    let mut sup = MempoolSupervisor::new(provider.clone(), 1, Duration::from_secs(1), 10);
    sup.ingest_tx(sample_tx(1, 10.0, 0));
    tokio::time::sleep(Duration::from_millis(10)).await;
    *provider.block.lock().unwrap() = 1;
    let groups = sup.tick().await.unwrap();
    assert!(!groups.is_empty());
    assert_eq!(groups[0].metadata.window_id, 0);
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
    *provider.block.lock().unwrap() = 0;
    sup.tick().await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    *provider.block.lock().unwrap() = 1;
    let groups = sup.tick().await.unwrap();
    assert_eq!(groups.len(), 2);
    let meta = &groups[0].metadata;
    assert_eq!(meta.timestamp_jitter_score, 0.0);
    assert_eq!(meta.window_id, 0);
    assert!(meta.state_alignment_score > 0.5);
}

#[tokio::test]
async fn confidence_penalty_overlap() {
    let _ = std::fs::remove_dir_all("snapshot_db");
    let provider = DummyProvider::default();
    let mut sup = MempoolSupervisor::new(provider.clone(), 1, Duration::from_secs(1), 10);
    let tx = sample_tx(1, 10.0, 1);
    sup.ingest_tx(tx.clone());
    *provider.block.lock().unwrap() = 0;
    sup.tick().await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    *provider.block.lock().unwrap() = 1;
    sup.tick().await.unwrap();
    *provider.block.lock().unwrap() = 2;
    let groups = sup.tick().await.unwrap();
    assert!((groups[0].group.txs[0].confidence - 0.9).abs() < 1e-6);
}

#[tokio::test]
async fn memory_pressure_graceful_degradation() {
    use rlimit::{Resource, getrlimit, setrlimit};

    // impose a memory limit but keep it large enough to avoid OOM in CI
    let limit = 512 * 1024 * 1024;
    let (old_soft, old_hard) = getrlimit(Resource::AS).unwrap();
    setrlimit(Resource::AS, limit, limit).expect("setrlimit failed");

    let provider = DummyProvider::default();
    let mut sup = MempoolSupervisor::new(provider.clone(), 1, Duration::from_secs(1), 10);

    let heavy_targets = vec![Address::repeat_byte(0xbb); 300];
    for i in 0u32..10_000u32 {
        let tx = AnnotatedTx {
            tx_hash: H256::from_low_u64_be(i as u64),
            token_paths: vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)],
            targets: heavy_targets.clone(),
            tags: vec!["swap-v2".to_string()],
            first_seen: i as u64,
            gas_price: 10.0,
            max_priority_fee_per_gas: None,
            confidence: 1.0,
        };
        sup.ingest_tx(tx);
    }

    *provider.block.lock().unwrap() = 1;
    // tick should succeed within timeout without panicking
    let res = tokio::time::timeout(Duration::from_secs(5), sup.tick()).await;
    assert!(res.is_ok() && res.unwrap().is_ok());

    // restore original limits
    let _ = setrlimit(Resource::AS, old_soft, old_hard);
}
