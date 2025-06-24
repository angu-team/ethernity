use ethernity_detector_mev::{
    AttackDetector, TxNatureTagger, TxAggregator, StateSnapshotRepository,
    StateImpactEvaluator, SnapshotProfile, RawTx
};
use ethernity_core::{traits::RpcProvider, error::{Result, Error}};
use ethereum_types::{Address, H256, U256};
use async_trait::async_trait;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tempfile::TempDir;

#[derive(Clone)]
struct FlakyProvider {
    fail_code: Arc<AtomicBool>,
    fail_hash: Arc<AtomicBool>,
}

#[async_trait]
impl RpcProvider for FlakyProvider {
    async fn get_transaction_trace(&self, _tx_hash: H256) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_transaction_receipt(&self, _tx_hash: H256) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_code(&self, _address: Address) -> Result<Vec<u8>> {
        if self.fail_code.swap(false, Ordering::SeqCst) {
            Err(Error::RpcError("code fail".into()))
        } else {
            Ok(vec![0xf4])
        }
    }
    async fn call(&self, _to: Address, _data: Vec<u8>) -> Result<Vec<u8>> {
        let mut out = vec![0u8; 96];
        U256::from(1000u64).to_big_endian(&mut out[0..32]);
        U256::from(1000u64).to_big_endian(&mut out[32..64]);
        Ok(out)
    }
    async fn get_block_number(&self) -> Result<u64> { Ok(1) }
    async fn get_block_hash(&self, _block_number: u64) -> Result<H256> {
        if self.fail_hash.swap(false, Ordering::SeqCst) {
            Err(Error::RpcError("hash fail".into()))
        } else {
            Ok(H256::repeat_byte(0x01))
        }
    }
}

fn make_input() -> Vec<u8> {
    let mut data = vec![0x38, 0xed, 0x17, 0x39];
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(Address::repeat_byte(0x01).as_bytes());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(Address::repeat_byte(0x02).as_bytes());
    data
}

#[tokio::test]
async fn integration_full_pipeline() {
    let provider = FlakyProvider {
        fail_code: Arc::new(AtomicBool::new(true)),
        fail_hash: Arc::new(AtomicBool::new(true)),
    };

    let tagger = TxNatureTagger::new(provider.clone());
    let mut aggr = TxAggregator::new();

    let to = Address::repeat_byte(0xaa);
    let txs = vec![
        RawTx { // will fail due to get_code error
            tx_hash: H256::repeat_byte(0x10),
            to,
            input: make_input(),
            first_seen: 1,
            gas_price: 20.0,
            max_priority_fee_per_gas: Some(2.0),
        },
        RawTx { // valid
            tx_hash: H256::repeat_byte(0x11),
            to,
            input: make_input(),
            first_seen: 2,
            gas_price: 20.0,
            max_priority_fee_per_gas: Some(2.0),
        },
        RawTx { // malformed
            tx_hash: H256::repeat_byte(0x12),
            to,
            input: vec![1,2,3],
            first_seen: 3,
            gas_price: 15.0,
            max_priority_fee_per_gas: Some(1.0),
        },
        RawTx { // valid second
            tx_hash: H256::repeat_byte(0x13),
            to,
            input: make_input(),
            first_seen: 4,
            gas_price: 10.0,
            max_priority_fee_per_gas: Some(1.0),
        }
    ];

    for raw in txs {
        if let Ok(res) = tagger.analyze(raw.to, &raw.input, raw.tx_hash).await {
            aggr.add_tx(ethernity_detector_mev::AnnotatedTx {
                tx_hash: raw.tx_hash,
                token_paths: res.token_paths,
                targets: res.targets,
                tags: res.tags,
                first_seen: raw.first_seen,
                gas_price: raw.gas_price,
                max_priority_fee_per_gas: raw.max_priority_fee_per_gas,
                confidence: res.confidence,
            });
        }
    }

    assert_eq!(aggr.groups().len(), 1);
    let group = aggr.groups().values().next().unwrap();
    assert_eq!(group.txs.len(), 2); // one dropped due to rpc error, one malformed

    let dir = TempDir::new().unwrap();
    let repo = StateSnapshotRepository::open(provider.clone(), dir.path()).unwrap();
    let err = repo
        .snapshot_groups(aggr.groups(), 1, SnapshotProfile::Basic)
        .await
        .unwrap_err();
    match err {
        Error::RpcError(_) => {},
        _ => panic!("unexpected error"),
    }
    // second attempt succeeds (provider error cleared)
    repo
        .snapshot_groups(aggr.groups(), 1, SnapshotProfile::Basic)
        .await
        .unwrap();
    let snap = repo
        .get_state(to, 1, SnapshotProfile::Basic)
        .expect("snapshot present");

    let impact = StateImpactEvaluator::evaluate(group, &[], &snap);
    let verdict = AttackDetector::new(1.0, 10)
        .analyze_group(group)
        .expect("verdict");
    assert_eq!(verdict.group_key, group.group_key);
    assert!(matches!(verdict.attack_type, Some(_)));
}

