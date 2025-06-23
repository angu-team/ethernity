use ethernity_detector_mev::{AnnotatedTx, TxAggregator};
use ethereum_types::{Address, H256};

#[tokio::test]
async fn aggregate_simple() {
    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];

    let tx1 = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x10),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 10.0,
        max_priority_fee_per_gas: None,
        confidence: 0.8,
    };
    let tx2 = AnnotatedTx { tx_hash: H256::repeat_byte(0x11), first_seen: 2, gas_price: 9.0, ..tx1.clone() };

    aggr.add_tx(tx1);
    aggr.add_tx(tx2);

    assert_eq!(aggr.groups().len(), 1);
    let group = aggr.groups().values().next().unwrap();
    assert_eq!(group.txs.len(), 2);
    assert!(!group.reorderable);
}
