use ethernity_detector_mev::{AnnotatedTx, TxAggregator, AttackDetector, AttackType};
use ethereum_types::{Address, H256};

#[test]
fn detect_basic_frontrun() {
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
        gas_price: 20.0,
        max_priority_fee_per_gas: Some(2.0),
        confidence: 0.9,
    };
    let tx2 = AnnotatedTx { tx_hash: H256::repeat_byte(0x11), first_seen: 2, gas_price: 10.0, max_priority_fee_per_gas: Some(1.0), ..tx1.clone() };

    aggr.add_tx(tx1);
    aggr.add_tx(tx2);

    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(1.0, 10);
    let res = detector.analyze_group(group).expect("should detect");
    match res.attack_type {
        Some(AttackType::Frontrun { .. }) => {}
        _ => panic!("expected frontrun"),
    }
}

#[test]
fn detect_basic_sandwich() {
    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];

    let tx1 = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x21),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 20.0,
        max_priority_fee_per_gas: Some(2.0),
        confidence: 0.9,
    };
    let tx2 = AnnotatedTx { tx_hash: H256::repeat_byte(0x22), first_seen: 2, gas_price: 10.0, max_priority_fee_per_gas: Some(1.0), ..tx1.clone() };
    let tx3 = AnnotatedTx { tx_hash: H256::repeat_byte(0x23), first_seen: 3, gas_price: 19.0, max_priority_fee_per_gas: Some(2.0), ..tx1.clone() };

    aggr.add_tx(tx1);
    aggr.add_tx(tx2);
    aggr.add_tx(tx3);

    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(1.0, 10);
    let res = detector.analyze_group(group).expect("should detect");
    match res.attack_type {
        Some(AttackType::Sandwich { .. }) => {}
        _ => panic!("expected sandwich"),
    }
}


