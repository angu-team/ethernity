use ethernity_detector_mev::{AnnotatedTx, TxAggregator, AttackDetector, AttackType};
use ethereum_types::{Address, H256};

#[test]
fn detect_cross_chain_multi_token() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![
        Address::repeat_byte(0x01),
        Address::repeat_byte(0x02),
        Address::repeat_byte(0x03),
    ];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["router-call".to_string(), "cross-chain".to_string()];

    let base = AnnotatedTx {
        tx_hash: H256::repeat_byte(0xc1),
        token_paths: tokens.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 20.0,
        max_priority_fee_per_gas: Some(2.0),
        confidence: 0.9,
    };
    let mut tx2 = base.clone();
    tx2.tx_hash = H256::repeat_byte(0xc2);
    tx2.first_seen = 2;
    aggr.add_tx(base);
    aggr.add_tx(tx2);

    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(1.0, 10);
    let res = detector.analyze_group(group).expect("should detect cross-chain");
    assert!(res.attack_types.iter().any(|a| matches!(a, AttackType::CrossChain { .. })));
}

#[test]
fn detect_flash_loan_attack() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xbb)];
    let tags = vec!["router-call".to_string(), "flash-loan".to_string()];

    let tx1 = AnnotatedTx {
        tx_hash: H256::repeat_byte(0xd1),
        token_paths: tokens.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 100.0,
        max_priority_fee_per_gas: Some(10.0),
        confidence: 0.9,
    };
    let mut tx2 = tx1.clone();
    tx2.tx_hash = H256::repeat_byte(0xd2);
    tx2.first_seen = 2;
    aggr.add_tx(tx1);
    aggr.add_tx(tx2);

    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(1.0, 10);
    let res = detector.analyze_group(group).expect("should detect flash loan");
    assert!(res.attack_types.iter().any(|a| matches!(a, AttackType::FlashLoan { .. })));
}

#[test]
fn detect_multi_token_attack() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![
        Address::repeat_byte(0x01),
        Address::repeat_byte(0x02),
        Address::repeat_byte(0x03),
        Address::repeat_byte(0x04),
    ];
    let targets = vec![Address::repeat_byte(0xcc)];
    let tags = vec!["swap-v2".to_string()];

    let tx1 = AnnotatedTx {
        tx_hash: H256::repeat_byte(0xe1),
        token_paths: tokens.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 20.0,
        max_priority_fee_per_gas: Some(2.0),
        confidence: 0.9,
    };
    let mut tx2 = tx1.clone();
    tx2.tx_hash = H256::repeat_byte(0xe2);
    tx2.first_seen = 2;
    aggr.add_tx(tx1);
    aggr.add_tx(tx2);

    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(1.0, 10);
    let res = detector.analyze_group(group).expect("should detect multi-token");
    assert!(res.attack_types.iter().any(|a| matches!(a, AttackType::MultiToken { .. })));
}

#[test]
fn detect_layer2_mev() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xdd)];
    let tags = vec!["swap-v3".to_string(), "l2".to_string()];

    let tx1 = AnnotatedTx {
        tx_hash: H256::repeat_byte(0xf1),
        token_paths: tokens.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 30.0,
        max_priority_fee_per_gas: Some(3.0),
        confidence: 0.9,
    };
    let mut tx2 = tx1.clone();
    tx2.tx_hash = H256::repeat_byte(0xf2);
    tx2.first_seen = 2;
    aggr.add_tx(tx1);
    aggr.add_tx(tx2);

    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(1.0, 10);
    let res = detector.analyze_group(group).expect("should detect l2 mev");
    assert!(res.attack_types.iter().any(|a| matches!(a, AttackType::Layer2 { .. })));
}
