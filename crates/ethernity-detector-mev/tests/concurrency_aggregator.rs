#[cfg(loom)]
use loom::sync::{Arc, Mutex};
#[cfg(loom)]
use loom::thread;
#[cfg(not(loom))]
use std::sync::{Arc, Mutex};
#[cfg(not(loom))]
use std::thread;

use ethernity_detector_mev::{AnnotatedTx, TxAggregator};
use ethereum_types::{Address, H256};

fn sample_tx(idx: u8) -> AnnotatedTx {
    AnnotatedTx {
        tx_hash: H256::repeat_byte(idx),
        token_paths: vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)],
        targets: vec![Address::repeat_byte(0xaa)],
        tags: vec!["swap-v2".to_string()],
        first_seen: idx as u64,
        gas_price: 1.0,
        max_priority_fee_per_gas: None,
        confidence: 1.0,
    }
}

#[test]
fn concurrent_add_finalize() {
    #[cfg(loom)]
    {
        let mut builder = loom::model::Builder::new();
        builder.max_threads = 10;
        builder.check(|| {
            run_test(10, 2);
        });
    }
    #[cfg(not(loom))]
    {
        run_test(100, 20);
    }
}

fn run_test(add_threads: usize, finalize_threads: usize) {
    let aggr = Arc::new(Mutex::new(TxAggregator::new()));
    let mut handles = Vec::new();
    for i in 0..add_threads as u8 {
        let a = Arc::clone(&aggr);
        handles.push(thread::spawn(move || {
            let tx = sample_tx(i);
            let mut g = a.lock().unwrap();
            g.add_tx(tx);
        }));
    }
    for _ in 0..finalize_threads {
        let a = Arc::clone(&aggr);
        handles.push(thread::spawn(move || {
            let mut g = a.lock().unwrap();
            g.finalize_events(true);
        }));
    }
    for h in handles { h.join().unwrap(); }
    let g = aggr.lock().unwrap();
    assert_eq!(g.groups().len(), 1);
    let group = g.groups().values().next().unwrap();
    assert_eq!(group.txs.len(), add_threads);
}

fn unique_group_tx(idx: u64) -> AnnotatedTx {
    AnnotatedTx {
        tx_hash: H256::from_low_u64_be(idx),
        token_paths: vec![
            Address::from_low_u64_be(idx),
            Address::from_low_u64_be(idx.wrapping_add(1)),
        ],
        targets: vec![Address::repeat_byte(0xaa)],
        tags: vec!["swap-v2".to_string()],
        first_seen: idx,
        gas_price: 1.0,
        max_priority_fee_per_gas: None,
        confidence: 1.0,
    }
}

fn run_limit_test(thread_count: usize) {
    let aggr = Arc::new(Mutex::new(TxAggregator::new()));
    let mut handles = Vec::new();
    for i in 0..thread_count as u64 {
        let a = Arc::clone(&aggr);
        handles.push(thread::spawn(move || {
            let tx = unique_group_tx(i);
            let mut g = a.lock().unwrap();
            g.add_tx(tx);
            assert!(g.groups().len() <= TxAggregator::MAX_GROUPS);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let g = aggr.lock().unwrap();
    assert_eq!(g.groups().len(), TxAggregator::MAX_GROUPS);
}

#[test]
fn parallel_group_limit_enforcement() {
    #[cfg(loom)]
    {
        let mut builder = loom::model::Builder::new();
        builder.max_threads = 100;
        builder.check(|| {
            run_limit_test(100);
        });
    }
    #[cfg(not(loom))]
    {
        run_limit_test(10_000);
    }
}
