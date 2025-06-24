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

fn sample_tx(idx: u64, gas: f64, ts: u64) -> AnnotatedTx {
    AnnotatedTx {
        tx_hash: H256::from_low_u64_be(idx),
        token_paths: vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)],
        targets: vec![Address::repeat_byte(0xaa)],
        tags: vec!["swap-v2".to_string()],
        first_seen: ts,
        gas_price: gas,
        max_priority_fee_per_gas: None,
        confidence: 1.0,
    }
}

#[tokio::test]
async fn supervisor_mode_transitions() {
    let _ = std::fs::remove_dir_all("snapshot_db");
    let provider = DummyProvider::default();
    let mut sup = MempoolSupervisor::new(provider.clone(), 1, Duration::from_secs(1), 2000);

    for i in 0..10u64 { sup.ingest_tx(sample_tx(i, 10.0, i)); }
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(sup.tick().await.unwrap().is_empty());

    for i in 10u64..1010u64 { sup.ingest_tx(sample_tx(i, 10.0, i)); }
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(sup.tick().await.unwrap().is_empty());

    let late = sample_tx(200u64, 10.0, 200);
    sup.ingest_tx(late);
    tokio::time::sleep(Duration::from_secs(4)).await; // exceeds burst TTL
    assert!(sup.tick().await.unwrap().is_empty());

    *provider.block.lock().unwrap() = 1;
    let groups = sup.tick().await.unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].group.txs.len(), 1020); // late tx expired, originals duplicated
}
