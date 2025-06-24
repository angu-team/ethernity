use ethernity_detector_mev::{TxNatureTagger, events::RawTx, traits::{TransactionClassifier, TagPrediction}};
use ethernity_core::{traits::RpcProvider, error::{Result, Error}, types::TransactionHash};
use ethereum_types::{Address, H256};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct CountingProvider {
    calls: Arc<Mutex<usize>>, 
    code: Vec<u8>,
    fail: bool,
}

impl CountingProvider {
    fn new(code: Vec<u8>) -> Self { Self { calls: Arc::new(Mutex::new(0)), code, fail: false } }
    fn failing() -> Self { Self { calls: Arc::new(Mutex::new(0)), code: vec![], fail: true } }
    fn count(&self) -> usize { *self.calls.lock().unwrap() }
}

#[async_trait]
impl RpcProvider for CountingProvider {
    async fn get_transaction_trace(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_transaction_receipt(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_code(&self, _address: Address) -> Result<Vec<u8>> {
        if self.fail { return Err(Error::Other("fail".into())); }
        let mut c = self.calls.lock().unwrap();
        *c += 1;
        Ok(self.code.clone())
    }
    async fn call(&self, _to: Address, _data: Vec<u8>) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_block_number(&self) -> Result<u64> { Ok(0) }
    async fn get_block_hash(&self, _block_number: u64) -> Result<H256> { Ok(H256::zero()) }
}

fn padded(addr: Address) -> Vec<u8> {
    let mut v = vec![0u8; 32];
    v[12..].copy_from_slice(&addr.0);
    v
}

#[tokio::test]
async fn known_selector_detection() {
    let provider = CountingProvider::new(vec![0x60,0x00,0x56]);
    let tagger = TxNatureTagger::new(provider);
    let to = Address::repeat_byte(1);
    let tx = H256::zero();

    let data_v2 = hex::decode("38ed1739").unwrap();
    let res = tagger.analyze(to, &data_v2, tx).await.unwrap();
    assert!(res.tags.contains(&"swap-v2".to_string()));

    let data_v3 = hex::decode("18cbaf95").unwrap();
    let res = tagger.analyze(to, &data_v3, tx).await.unwrap();
    assert!(res.tags.contains(&"swap-v3".to_string()));

    let data_transfer = hex::decode("a9059cbb").unwrap();
    let res = tagger.analyze(to, &data_transfer, tx).await.unwrap();
    assert!(res.tags.contains(&"transfer".to_string()));
}

#[tokio::test]
async fn empty_and_unknown_selectors() {
    let provider = CountingProvider::new(vec![]);
    let tagger = TxNatureTagger::new(provider);
    let to = Address::zero();
    let tx = H256::zero();

    let res = tagger.analyze(to, &[], tx).await.unwrap();
    assert!(res.tags.is_empty());
    assert!(res.path_inference_failed);

    let unknown = hex::decode("deadbeef").unwrap();
    let res = tagger.analyze(to, &unknown, tx).await.unwrap();
    assert!(res.tags.is_empty());
    assert!(res.confidence_components.abi_match < 0.2);
}

#[tokio::test]
async fn bytecode_cache_hit_and_overflow() {
    let provider = CountingProvider::new(vec![0x60,0x00,0x56]);
    let tagger = TxNatureTagger::new(provider.clone());
    let tx = H256::zero();

    let addr0 = Address::from_low_u64_be(0);
    tagger.analyze(addr0, &[], tx).await.unwrap();
    assert_eq!(provider.count(), 1);
    tagger.analyze(addr0, &[], tx).await.unwrap();
    assert_eq!(provider.count(), 1); // cache hit

    for i in 1..1025 {
        let addr = Address::from_low_u64_be(i as u64);
        tagger.analyze(addr, &[], tx).await.unwrap();
    }
    assert_eq!(provider.count(), 1025);

    tagger.analyze(addr0, &[], tx).await.unwrap();
    assert_eq!(provider.count(), 1026); // evicted and fetched again
}

#[tokio::test]
async fn edge_cases_parsing() {
    let provider = CountingProvider::new(vec![0x60,0xf4,0x56]);
    let tagger = TxNatureTagger::new(provider);
    let to = Address::repeat_byte(2);
    let tx = H256::zero();

    let a1 = Address::repeat_byte(0x11);
    let a2 = Address::repeat_byte(0x22);
    let mut data = hex::decode("38ed1739").unwrap();
    data.extend(padded(a1));
    data.extend(vec![0u8;32]);
    data.extend(padded(a2));
    data.extend(vec![1u8;16]); // incomplete chunk

    let res = tagger.analyze(to, &data, tx).await.unwrap();
    assert!(res.tags.contains(&"proxy-call".to_string()));
    assert_eq!(res.token_paths, vec![a1,a2]);
    assert!(res.extracted_fallback);
    assert!(!res.path_inference_failed);
}

#[tokio::test]
async fn confidence_components_and_flags() {
    let provider = CountingProvider::new(vec![0x60,0xf4,0x56]);
    let tagger = TxNatureTagger::new(provider);
    let to = Address::repeat_byte(3);
    let tx = H256::zero();

    let a1 = Address::repeat_byte(0x11);
    let mut data = hex::decode("38ed1739").unwrap();
    data.extend(padded(a1));

    let res = tagger.analyze(to, &data, tx).await.unwrap();
    assert_eq!(res.confidence_components.abi_match, 0.9);
    assert_eq!(res.confidence_components.structure, 0.7);
    assert_eq!(res.confidence_components.path, 0.5);
    assert_eq!(res.confidence, (0.9+0.7+0.5)/3.0);
    assert!(res.extracted_fallback);
    assert!(!res.path_inference_failed);

    let provider2 = CountingProvider::new(vec![0x60,0x00,0x56]);
    let tagger2 = TxNatureTagger::new(provider2);
    let unknown = hex::decode("ffffffff").unwrap();
    let res2 = tagger2.analyze(to, &unknown, tx).await.unwrap();
    assert_eq!(res2.confidence_components.abi_match, 0.1);
    assert_eq!(res2.confidence_components.structure, 0.5);
    assert_eq!(res2.confidence_components.path, 0.0);
    assert!(res2.path_inference_failed);
}

#[tokio::test]
async fn process_stream_and_trait_impl() {
    let provider = CountingProvider::new(vec![0x60,0x00,0x56]);
    let tagger = TxNatureTagger::new(provider);
    let (tx_in, rx_in) = tokio::sync::mpsc::channel(4);
    let (tx_out, mut rx_out) = tokio::sync::mpsc::channel(4);

    let handle = tokio::spawn(async move { tagger.process_stream(rx_in, tx_out).await });

    let data = hex::decode("a9059cbb").unwrap();
    let raw = RawTx { tx_hash: H256::zero(), to: Address::zero(), input: data.clone(), first_seen: 1, gas_price: 1.0, max_priority_fee_per_gas: None };
    tx_in.send(raw).await.unwrap();
    drop(tx_in);

    let annotated = rx_out.recv().await.unwrap();
    assert!(annotated.tags.contains(&"transfer".to_string()));

    handle.await.unwrap();

    // test trait implementation
    let provider2 = CountingProvider::new(vec![0x60,0x00,0x56]);
    let tagger2 = TxNatureTagger::new(provider2);
    let preds = tagger2.classify(Address::zero(), &data, H256::zero()).await.unwrap();
    assert_eq!(preds.len(), 2); // transfer + token-move
}

#[tokio::test]
async fn rpc_failure_propagates() {
    let provider = CountingProvider::failing();
    let tagger = TxNatureTagger::new(provider);
    let err = tagger.analyze(Address::zero(), &[], H256::zero()).await;
    assert!(err.is_err());
}

