use ethernity_detector_mev::{AnnotatedTx, TxAggregator, AggregationEvent};
use ethereum_types::{Address, H256};
use tokio::sync::mpsc;

fn make_tx(hash_byte: u8, first_seen: u64, gas: f64, confidence: f64, tokens: Vec<Address>, targets: Vec<Address>, tags: Vec<String>) -> AnnotatedTx {
    AnnotatedTx {
        tx_hash: H256::repeat_byte(hash_byte),
        token_paths: tokens,
        targets,
        tags,
        first_seen,
        gas_price: gas,
        max_priority_fee_per_gas: None,
        confidence,
    }
}

#[test]
fn grouping_same_inputs() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let tx1 = make_tx(0x10, 1, 10.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let tx2 = make_tx(0x11, 2, 9.0, 0.8, tokens.clone(), targets.clone(), tags.clone());
    aggr.add_tx(tx1);
    aggr.add_tx(tx2);
    assert_eq!(aggr.groups().len(), 1);
    assert_eq!(aggr.groups().values().next().unwrap().txs.len(), 2);
}

#[test]
fn grouping_different_inputs() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets_a = vec![Address::repeat_byte(0xaa)];
    let targets_b = vec![Address::repeat_byte(0xbb)];
    let tags = vec!["swap-v2".to_string()];
    let tx1 = make_tx(0x20, 1, 10.0, 0.9, tokens.clone(), targets_a, tags.clone());
    let tx2 = make_tx(0x21, 1, 10.0, 0.9, tokens.clone(), targets_b, tags.clone());
    aggr.add_tx(tx1);
    aggr.add_tx(tx2);
    assert_eq!(aggr.groups().len(), 2);
}

#[test]
fn insertion_order_sorted() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    // add later first_seen first, then earlier
    let tx_late = make_tx(0x30, 5, 5.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let tx_early = make_tx(0x31, 1, 20.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    aggr.add_tx(tx_late);
    aggr.add_tx(tx_early);
    let group = aggr.groups().values().next().unwrap();
    assert_eq!(group.txs.first().unwrap().tx_hash, H256::repeat_byte(0x31));
    assert_eq!(group.txs.last().unwrap().tx_hash, H256::repeat_byte(0x30));
}

#[test]
fn group_key_deterministic() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let tx1 = make_tx(0x40, 1, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let tx2 = make_tx(0x41, 2, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let key1 = aggr.add_tx(tx1).unwrap();
    let key2 = aggr.add_tx(tx2).unwrap();
    assert_eq!(key1, key2);
}

#[test]
fn filter_insufficient_tokens() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let tx = make_tx(0x50, 1, 1.0, 0.9, tokens, targets, tags);
    assert!(aggr.add_tx(tx).is_none());
    assert_eq!(aggr.groups().len(), 0);
}

#[test]
fn filter_no_targets() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let tags = vec!["swap-v2".to_string()];
    let tx = make_tx(0x51, 1, 1.0, 0.9, tokens, vec![], tags);
    assert!(aggr.add_tx(tx).is_none());
    assert_eq!(aggr.groups().len(), 0);
}

#[test]
fn filter_invalid_tags() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["unknown".to_string()];
    let tx = make_tx(0x52, 1, 1.0, 0.9, tokens, targets, tags);
    assert!(aggr.add_tx(tx).is_none());
    assert_eq!(aggr.groups().len(), 0);
}

#[test]
fn filter_low_confidence_after_two_high() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let high1 = make_tx(0x60, 1, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let high2 = make_tx(0x61, 2, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let low = make_tx(0x62, 3, 1.0, 0.4, tokens.clone(), targets.clone(), tags.clone());
    assert!(aggr.add_tx(high1).is_some());
    assert!(aggr.add_tx(high2).is_some());
    assert!(aggr.add_tx(low).is_some());
    let group = aggr.groups().values().next().unwrap();
    assert_eq!(group.txs.len(), 3);
}

#[test]
fn metric_ordering_certainty() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let a = make_tx(0x70, 0, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let b = make_tx(0x71, 10, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    aggr.add_tx(a);
    aggr.add_tx(b);
    let group = aggr.groups().values().next().unwrap();
    assert_eq!(group.ordering_certainty_score, 1.0);
    let c = make_tx(0x72, 50, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    aggr.add_tx(c);
    let group = aggr.groups().values().next().unwrap();
    assert_eq!(group.ordering_certainty_score, 0.7);
}

#[test]
fn metric_contamination_variance() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let t1 = make_tx(0x80, 1, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let t2 = make_tx(0x81, 2, 1.0, 0.3, tokens.clone(), targets.clone(), tags.clone());
    aggr.add_tx(t1);
    // second tx has low confidence and should be ignored
    aggr.add_tx(t2);
    let group = aggr.groups().values().next().unwrap();
    assert!(!group.contaminated);
    assert_eq!(group.txs.len(), 1);
}

#[test]
fn metric_direction_signature() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let tx = make_tx(0x90, 1, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    aggr.add_tx(tx);
    let group = aggr.groups().values().next().unwrap();
    let expected = format!("0x{:x}→0x{:x}", tokens[0], tokens[1]);
    assert_eq!(group.direction_signature, expected);
}

#[test]
fn aggregate_multiple_pools() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let tags = vec!["swap-v2".to_string()];
    for i in 0..3u8 {
        let targets = vec![Address::from_low_u64_be(i as u64)];
        let tx = make_tx(0x90 + i, i as u64, 1.0, 0.9, tokens.clone(), targets, tags.clone());
        aggr.add_tx(tx);
    }
    assert_eq!(aggr.groups().len(), 3);
}

#[test]
fn metric_reorderable_flag() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let t1 = make_tx(0xa0, 0, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let t2 = make_tx(0xa1, 100, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    aggr.add_tx(t1);
    aggr.add_tx(t2);
    let group = aggr.groups().values().next().unwrap();
    assert!(!group.reorderable);
}


#[test]
fn events_add_tx_event() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let tx = make_tx(0xb0, 1, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let ev = aggr.add_tx_event(tx.clone()).unwrap();
    match ev {
        AggregationEvent::PartialGroup { group_key, txs, window_start } => {
            assert_eq!(txs[0].tx_hash, tx.tx_hash);
            let group = aggr.groups().get(&group_key).unwrap();
            assert_eq!(group.window_start, window_start);
        }
        _ => panic!("expected PartialGroup"),
    }
}

#[test]
fn events_finalize_events() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let tx = make_tx(0xc0, 1, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone());
    let key = aggr.add_tx(tx).unwrap();
    let events = aggr.finalize_events(false);
    assert_eq!(events.len(), 1);
    match &events[0] {
        AggregationEvent::FinalizedGroup { group_key, complete } => {
            assert_eq!(key, *group_key);
            assert!(!complete);
        }
        _ => panic!("expected FinalizedGroup"),
    }
}

#[tokio::test]
async fn events_process_stream_events() {
    let aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];
    let (tx_in, rx_in) = mpsc::channel(4);
    let (tx_out, mut rx_out) = mpsc::channel(4);
    tokio::spawn(async move { aggr.process_stream_events(rx_in, tx_out).await; });
    tx_in.send(make_tx(0xd0, 1, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone())).await.unwrap();
    tx_in.send(make_tx(0xd1, 2, 1.0, 0.9, tokens.clone(), targets.clone(), tags.clone())).await.unwrap();
    drop(tx_in);
    let mut events = Vec::new();
    while let Some(ev) = rx_out.recv().await { events.push(ev); }
    assert_eq!(events.len(), 3); // two partial + one finalized
    assert!(matches!(events.last().unwrap(), AggregationEvent::FinalizedGroup { complete: true, .. }));
}

#[test]
fn group_count_hard_limit_enforcement() {
    let mut aggr = TxAggregator::new();
    let target = Address::repeat_byte(0xaa);
    let tags = vec!["swap-v2".to_string()];

    let mut first_key = H256::zero();
    let mut last_key = H256::zero();
    // create 100k unique groups
    for i in 0..100_000u64 {
        let tokens = vec![
            Address::from_low_u64_be(i),
            Address::from_low_u64_be(i.wrapping_add(1)),
        ];
        let tx = make_tx(
            (i % 255) as u8,
            i,
            1.0,
            0.9,
            tokens.clone(),
            vec![target],
            tags.clone(),
        );
        let key = aggr.add_tx(tx).unwrap();
        if i == 0 { first_key = key; }
        last_key = key;
    }

    assert_eq!(aggr.groups().len(), TxAggregator::MAX_GROUPS);
    assert!(!aggr.groups().contains_key(&first_key));
    assert!(aggr.groups().contains_key(&last_key));
}

#[test]
fn massive_timestamp_collision_stability() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v2".to_string()];

    // insert 1000 transactions with the same timestamp but varying gas prices
    for i in (0..1000u64).rev() {
        let gas = i as f64;
        let tx = make_tx((i % 255) as u8, 1234567890, gas, 0.9, tokens.clone(), targets.clone(), tags.clone());
        aggr.add_tx(tx);
    }

    let group = aggr.groups().values().next().unwrap();
    assert_eq!(group.txs.len(), 1000);
    // should remain deterministic: sorted by gas_price since timestamps equal
    for (idx, tx) in group.txs.iter().enumerate() {
        assert_eq!(tx.gas_price, idx as f64);
    }
    assert_eq!(group.ordering_certainty_score, 1.0);
    assert!(!group.reorderable);
}

