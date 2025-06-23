use ethernity_detector_mev::{AnnotatedTx, TxAggregator, VictimInput, StateSnapshot, StateImpactEvaluator};
use ethereum_types::{Address, H256};

#[test]
fn evaluate_basic() {
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

    let key = aggr.add_tx(tx.clone()).unwrap();
    let group = aggr.groups().get(&key).unwrap();

    let victims = vec![VictimInput {
        tx_hash: tx.tx_hash,
        amount_in: 100.0,
        amount_out_min: 90.0,
        token_behavior_unknown: false,
    }];

    let snapshot = StateSnapshot {
        reserve_in: 1000.0,
        reserve_out: 1000.0,
        sqrt_price_x96: None,
        liquidity: None,
        state_lag_blocks: 1,
        reorg_risk_level: "medium".to_string(),
        volatility_flag: false,
    };

    let result = StateImpactEvaluator::evaluate(group, &victims, &snapshot);
    assert_eq!(result.victims.len(), 1);
    assert!(result.opportunity_score > 0.0);
}
