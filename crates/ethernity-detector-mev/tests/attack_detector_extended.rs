use ethernity_detector_mev::{AnnotatedTx, TxAggregator, AttackDetector, AttackType};
use ethereum_types::{Address, H256};

#[test]
fn detect_basic_spoof() {
    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];

    let base = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x30),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 10.0,
        max_priority_fee_per_gas: None,
        confidence: 0.9,
    };
    let mut tx2 = base.clone();
    tx2.tx_hash = H256::repeat_byte(0x31);
    tx2.first_seen = 2;
    let mut tx3 = base.clone();
    tx3.tx_hash = H256::repeat_byte(0x32);
    tx3.first_seen = 3;
    tx3.gas_price = 50.0;
    tx3.tags.push("long-long-long-long-long-tag".to_string());

    aggr.add_tx(base);
    aggr.add_tx(tx2);
    aggr.add_tx(tx3);

    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(1.0, 10);
    let res = detector.analyze_group(group).expect("should detect");
    match res.attack_type {
        Some(AttackType::Spoof { .. }) => assert!(res.confidence >= 0.5),
        _ => panic!("expected spoof"),
    }
}

#[test]
fn detect_basic_backrun() {
    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];

    let base = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x40),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 15.0,
        max_priority_fee_per_gas: None,
        confidence: 0.9,
    };
    let mut tx2 = base.clone();
    tx2.tx_hash = H256::repeat_byte(0x41);
    tx2.first_seen = 2;
    let mut tx3 = base.clone();
    tx3.tx_hash = H256::repeat_byte(0x42);
    tx3.first_seen = 3;
    tx3.gas_price = 40.0;

    aggr.add_tx(base);
    aggr.add_tx(tx2);
    aggr.add_tx(tx3);

    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(0.0, 10);
    let res = detector.analyze_group(group).expect("should detect");
    match res.attack_type {
        Some(AttackType::Backrun { .. }) => {},
        _ => panic!("expected backrun"),
    }
}

#[test]
fn edge_single_tx_group() {
    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];

    let tx = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x50),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 10.0,
        max_priority_fee_per_gas: None,
        confidence: 0.9,
    };

    aggr.add_tx(tx);
    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(1.0, 10);
    assert!(detector.analyze_group(group).is_none());
}

#[test]
fn edge_multiple_attacks_prefers_sandwich() {
    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];

    let mut a = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x60),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 30.0,
        max_priority_fee_per_gas: Some(3.0),
        confidence: 0.9,
    };
    let mut b = a.clone();
    b.tx_hash = H256::repeat_byte(0x61);
    b.first_seen = 2;
    b.gas_price = 10.0;
    b.max_priority_fee_per_gas = Some(1.0);
    let mut c = a.clone();
    c.tx_hash = H256::repeat_byte(0x62);
    c.first_seen = 3;
    let mut d = a.clone();
    d.tx_hash = H256::repeat_byte(0x63);
    d.first_seen = 4;
    d.gas_price = 40.0;

    aggr.add_tx(a);
    aggr.add_tx(b);
    aggr.add_tx(c);
    aggr.add_tx(d);

    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(0.0, 10);
    let res = detector.analyze_group(group).expect("should detect");
    match res.attack_type {
        Some(AttackType::Sandwich { .. }) => {},
        _ => panic!("expected sandwich first"),
    }
}

#[test]
fn edge_low_confidence_reconsiderable() {
    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    // long tags to trigger anomaly
    let tags = vec!["swap-v2".to_string(), "very-very-long-tag-over-twenty".to_string(), "another-very-long-tag".to_string()];

    let mut tx1 = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x70),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 10.0,
        max_priority_fee_per_gas: None,
        confidence: 0.9,
    };
    let mut tx2 = tx1.clone();
    tx2.tx_hash = H256::repeat_byte(0x71);
    tx2.first_seen = 2;
    tx2.tags = tags.clone();

    aggr.add_tx(tx1);
    aggr.add_tx(tx2);
    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(1.0, 10);
    let res = detector.analyze_group(group).expect("should detect");
    assert!(res.reconsiderable);
}

#[test]
fn priority_max_fee_respected() {
    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];

    let mut a = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x80),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 1,
        gas_price: 30.0,
        max_priority_fee_per_gas: Some(1.0),
        confidence: 0.9,
    };
    let mut b = a.clone();
    b.tx_hash = H256::repeat_byte(0x81);
    b.first_seen = 2;
    b.max_priority_fee_per_gas = None;
    b.gas_price = 20.0;

    aggr.add_tx(a);
    aggr.add_tx(b);
    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(10.0, 10);
    let res = detector.analyze_group(group).expect("should detect");
    match res.attack_type {
        Some(AttackType::Frontrun { .. }) => {},
        _ => panic!("expected frontrun"),
    }
}

#[test]
fn priority_ordering_by_seen() {
    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];

    let mut a = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x90),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: tags.clone(),
        first_seen: 2,
        gas_price: 10.0,
        max_priority_fee_per_gas: None,
        confidence: 0.9,
    };
    let mut b = a.clone();
    b.tx_hash = H256::repeat_byte(0x91);
    b.first_seen = 1;
    b.gas_price = 11.0;

    aggr.add_tx(a);
    aggr.add_tx(b);
    let group = aggr.groups().values().next().unwrap();
    // first tx in sorted group should be b
    assert_eq!(group.txs.first().unwrap().tx_hash, H256::repeat_byte(0x91));
}

