use ethernity_detector_mev::{traits::{StateProvider, TransactionClassifier, ImpactModel, TagPrediction}, AnnotatedTx, TxGroup, VictimInput, StateSnapshot, GroupImpact};
use ethernity_core::error::{Result, Error};
use ethernity_core::types::TransactionHash;
use ethereum_types::{Address, H256, U256};
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Default, Clone)]
struct MockStateProvider {
    reserves: HashMap<Address, (U256, U256)>,
    slots: HashMap<Address, (U256, U256)>,
}

#[async_trait]
impl StateProvider for MockStateProvider {
    async fn reserves(&self, address: Address) -> Result<(U256, U256)> {
        self.reserves
            .get(&address)
            .cloned()
            .ok_or_else(|| Error::NotFound("reserves".into()))
    }

    async fn slot0(&self, address: Address) -> Result<(U256, U256)> {
        self.slots
            .get(&address)
            .cloned()
            .ok_or_else(|| Error::NotFound("slot0".into()))
    }
}

#[derive(Clone)]
struct MockClassifier;

#[async_trait]
impl TransactionClassifier for MockClassifier {
    async fn classify(&self, _to: Address, _data: &[u8], _tx: TransactionHash) -> Result<Vec<TagPrediction>> {
        Ok(vec![TagPrediction { tag: "mock".into(), confidence: 1.0 }])
    }
}

struct MockImpactModel;

impl ImpactModel for MockImpactModel {
    fn evaluate_group(&self, group: &TxGroup, _victims: &[VictimInput], _snapshot: &StateSnapshot) -> GroupImpact {
        GroupImpact {
            group_id: group.group_key,
            tokens: group.token_paths.clone(),
            victims: Vec::new(),
            opportunity_score: 0.0,
            expected_profit_backrun: 0.0,
            state_confidence: 1.0,
            impact_certainty: 1.0,
            execution_assumption: "mock".to_string(),
            reorg_risk_level: "low".to_string(),
        }
    }
}

async fn use_state_provider<P: StateProvider>(provider: &P, addr: Address) -> Result<(U256, U256)> {
    provider.reserves(addr).await
}

async fn use_classifier<C: TransactionClassifier>(c: &C, addr: Address) -> Result<Vec<TagPrediction>> {
    c.classify(addr, &[], H256::zero()).await
}

#[tokio::test]
async fn mock_state_provider_works() {
    let addr = Address::repeat_byte(0x11);
    let mut mock = MockStateProvider::default();
    mock.reserves.insert(addr, (U256::from(1u64), U256::from(2u64)));
    mock.slots.insert(addr, (U256::from(3u64), U256::from(4u64)));
    let boxed: Box<dyn StateProvider> = Box::new(mock.clone());
    let (a,b) = boxed.reserves(addr).await.unwrap();
    assert_eq!(a, U256::from(1u64));
    assert_eq!(b, U256::from(2u64));
    let (a2, _) = use_state_provider(&mock, addr).await.unwrap();
    assert_eq!(a2, U256::from(1u64));
}

#[tokio::test]
async fn mock_classifier_works() {
    let cls = MockClassifier;
    let res = cls.classify(Address::repeat_byte(0x22), &[], H256::zero()).await.unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].tag, "mock");
    let res2 = use_classifier(&cls, Address::zero()).await.unwrap();
    assert_eq!(res2[0].tag, "mock");
}

#[test]
fn mock_impact_model_works() {
    let group = TxGroup {
        group_key: H256::repeat_byte(0x33),
        token_paths: vec![Address::repeat_byte(0x01)],
        targets: vec![],
        txs: vec![AnnotatedTx { tx_hash: H256::zero(), token_paths: vec![], targets: vec![], tags: vec![], first_seen: 0, gas_price: 0.0, max_priority_fee_per_gas: None, confidence: 0.0 }],
        block_number: None,
        direction_signature: String::new(),
        ordering_certainty_score: 0.0,
        reorderable: false,
        contaminated: false,
        window_start: 0,
    };
    let snapshot = StateSnapshot {
        reserve_in: 0.0,
        reserve_out: 0.0,
        sqrt_price_x96: None,
        liquidity: None,
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    };
    let impact = MockImpactModel.evaluate_group(&group, &[], &snapshot);
    assert_eq!(impact.group_id, group.group_key);
    assert!(impact.victims.is_empty());
}
