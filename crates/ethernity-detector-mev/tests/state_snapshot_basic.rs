use ethernity_detector_mev::{AnnotatedTx, TxAggregator, StateSnapshotRepository, SnapshotProfile};
use ethernity_core::{traits::RpcProvider, error::Result, types::TransactionHash};
use ethereum_types::{Address, H256, U256};
use async_trait::async_trait;

struct DummyProvider;

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
    async fn get_block_number(&self) -> Result<u64> { Ok(100) }
    async fn get_block_hash(&self, _block_number: u64) -> Result<H256> { Ok(H256::zero()) }
}

#[tokio::test]
async fn cache_basic_snapshot() {
    let provider = DummyProvider;
    let manager = StateSnapshotRepository::open(provider, "test_db").unwrap();

    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let tx = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x10),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 10.0,
        max_priority_fee_per_gas: None,
        confidence: 0.9,
    };
    aggr.add_tx(tx);

    manager
        .snapshot_groups(aggr.groups(), 101, SnapshotProfile::Basic)
        .await
        .unwrap();

    let target = Address::repeat_byte(0xaa);
    let snap = manager
        .get_state(target, 101, SnapshotProfile::Basic)
        .expect("snapshot missing");
    assert_eq!(snap.reserve_in, 1000.0);
    assert!(!snap.volatility_flag);
}
