use ethernity_detector_mev::TxNatureTagger;
use ethernity_core::{traits::RpcProvider, error::Result, types::TransactionHash};
use ethereum_types::{Address, H256};
use async_trait::async_trait;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

#[derive(Clone)]
struct CountingProvider {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl RpcProvider for CountingProvider {
    async fn get_transaction_trace(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_transaction_receipt(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_code(&self, _address: Address) -> Result<Vec<u8>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![0x60, 0x00, 0x56])
    }
    async fn call(&self, _to: Address, _data: Vec<u8>) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_block_number(&self) -> Result<u64> { Ok(0) }
    async fn get_block_hash(&self, _block_number: u64) -> Result<H256> { Ok(H256::zero()) }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn concurrent_cache_eviction_integrity() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = CountingProvider { calls: calls.clone() };
    let tagger = Arc::new(TxNatureTagger::new(provider));

    // fill cache to capacity
    for i in 0u64..1024 {
        let addr = Address::from_low_u64_be(i);
        tagger.analyze(addr, &[], H256::zero()).await.unwrap();
    }

    // spawn concurrent requests for new addresses (will trigger eviction)
    let mut handles = Vec::new();
    for i in 1024u64..1034 {
        let t = Arc::clone(&tagger);
        handles.push(tokio::spawn(async move {
            let addr = Address::from_low_u64_be(i);
            t.analyze(addr, &[], H256::zero()).await.unwrap();
        }));
    }
    for h in handles { h.await.unwrap(); }

    // each address should have been fetched exactly once
    assert_eq!(calls.load(Ordering::SeqCst), 1034);
}

