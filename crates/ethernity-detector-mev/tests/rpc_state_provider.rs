use ethernity_detector_mev::{RpcStateProvider, StateProvider};
use ethernity_core::{traits::RpcProvider, error::{Result, Error}, types::TransactionHash};
use ethereum_types::{Address, H256, U256};
use async_trait::async_trait;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

#[derive(Clone)]
struct CountingProvider {
    calls: Arc<AtomicUsize>,
    response: Vec<u8>,
    fail: bool,
}

#[async_trait]
impl RpcProvider for CountingProvider {
    async fn get_transaction_trace(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_transaction_receipt(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_code(&self, _address: Address) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn call(&self, _to: Address, _data: Vec<u8>) -> Result<Vec<u8>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err(Error::RpcError("fail".into()))
        } else {
            Ok(self.response.clone())
        }
    }
    async fn get_block_number(&self) -> Result<u64> { Ok(0) }
    async fn get_block_hash(&self, _block_number: u64) -> Result<H256> { Ok(H256::zero()) }
}

fn reserves_response(v0: u64, v1: u64) -> Vec<u8> {
    let mut out = vec![0u8; 96];
    U256::from(v0).to_big_endian(&mut out[0..32]);
    U256::from(v1).to_big_endian(&mut out[32..64]);
    out
}

#[tokio::test]
async fn cache_lru_hit_and_eviction() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = CountingProvider { calls: calls.clone(), response: reserves_response(1, 2), fail: false };
    let sp = RpcStateProvider::new(provider);
    // first address
    let addr0 = Address::repeat_byte(0xaa);
    sp.reserves(addr0).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // repeat hit
    sp.reserves(addr0).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // fill cache with 128 additional entries (exceed capacity by one)
    for i in 0u8..128u8 {
        let a = Address::repeat_byte(i);
        sp.reserves(a).await.unwrap();
    }
    assert_eq!(calls.load(Ordering::SeqCst), 129);
    // addr0 should be a miss now since it was the least recently used
    sp.reserves(addr0).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 130);
}

#[tokio::test]
async fn caches_are_independent() {
    let calls = Arc::new(AtomicUsize::new(0));
    let response = reserves_response(10, 20);
    let provider = CountingProvider { calls: calls.clone(), response: response.clone(), fail: false };
    let sp = RpcStateProvider::new(provider);
    let addr = Address::repeat_byte(0x01);
    sp.reserves(addr).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    sp.slot0(addr).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    sp.reserves(addr).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn fallback_primary_fail() {
    let calls_primary = Arc::new(AtomicUsize::new(0));
    let calls_fallback = Arc::new(AtomicUsize::new(0));
    let primary = CountingProvider { calls: calls_primary.clone(), response: reserves_response(5, 6), fail: true };
    let fallback = CountingProvider { calls: calls_fallback.clone(), response: reserves_response(5, 6), fail: false };
    let sp = RpcStateProvider::with_fallback(primary, fallback);
    let addr = Address::repeat_byte(0x11);
    let (r0, r1) = sp.reserves(addr).await.unwrap();
    assert_eq!((r0, r1), (U256::from(5u64), U256::from(6u64)));
    assert_eq!(calls_primary.load(Ordering::SeqCst), 1);
    assert_eq!(calls_fallback.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn fallback_both_fail() {
    let primary = CountingProvider { calls: Arc::new(AtomicUsize::new(0)), response: reserves_response(1, 2), fail: true };
    let fallback = CountingProvider { calls: Arc::new(AtomicUsize::new(0)), response: reserves_response(1, 2), fail: true };
    let sp = RpcStateProvider::with_fallback(primary, fallback);
    let addr = Address::repeat_byte(0x22);
    let res = sp.reserves(addr).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn no_fallback_configured() {
    let provider = CountingProvider { calls: Arc::new(AtomicUsize::new(0)), response: reserves_response(1, 2), fail: true };
    let sp = RpcStateProvider::new(provider);
    let addr = Address::repeat_byte(0x33);
    let res = sp.reserves(addr).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn parse_valid_and_invalid_responses() {
    // valid > 64 bytes
    let provider = CountingProvider { calls: Arc::new(AtomicUsize::new(0)), response: reserves_response(100, 200), fail: false };
    let sp = RpcStateProvider::new(provider);
    let addr = Address::repeat_byte(0x44);
    let (a, b) = sp.reserves(addr).await.unwrap();
    assert_eq!(a, U256::from(100u64));
    assert_eq!(b, U256::from(200u64));

    // invalid (<64 bytes)
    let short = vec![0u8; 60];
    let provider2 = CountingProvider { calls: Arc::new(AtomicUsize::new(0)), response: short.clone(), fail: false };
    let sp2 = RpcStateProvider::new(provider2);
    let addr2 = Address::repeat_byte(0x55);
    let res = sp2.reserves(addr2).await;
    assert!(matches!(res, Err(Error::DecodeError(_))));
}
