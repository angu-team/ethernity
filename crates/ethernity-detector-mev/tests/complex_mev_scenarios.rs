use std::sync::Arc;

use ethernity_detector_mev::{
    AnnotatedTx, TxAggregator, AttackDetector, AttackType, VictimInput, 
    StateImpactEvaluator, ImpactModelParams, ImpactModel, UniswapV3Curve, StateSnapshot
};
use ethereum_types::{Address, H256};

fn make_tx(
    hash: u8,
    first_seen: u64,
    gas_price: f64,
    priority: Option<f64>,
    tokens: &[Address],
    targets: &[Address],
    tags: &[String],
) -> AnnotatedTx {
    AnnotatedTx {
        tx_hash: H256::repeat_byte(hash),
        token_paths: tokens.to_vec(),
        targets: targets.to_vec(),
        tags: tags.to_vec(),
        first_seen,
        gas_price,
        max_priority_fee_per_gas: priority,
        confidence: 0.9,
    }
}

fn default_snapshot() -> StateSnapshot {
    StateSnapshot {
        reserve_in: 10_000.0,
        reserve_out: 10_000.0,
        sqrt_price_x96: Some(2_f64.powi(96)),
        liquidity: Some(100.0),
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    }
}

#[test]
fn detect_complex_multi_victim_sandwich() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v3".to_string()];

    // attacker initiates
    aggr.add_tx(make_tx(0xa0, 1, 50.0, Some(5.0), &tokens, &targets, &tags));
    // five victim transactions
    for i in 0..5u8 {
        aggr.add_tx(make_tx(0xa1 + i, (i as u64) + 2, 12.0, Some(1.0), &tokens, &targets, &tags));
    }
    // competing attacker in between
    aggr.add_tx(make_tx(0xb0, 7, 45.0, Some(4.0), &tokens, &targets, &tags));
    // attacker closes sandwich
    aggr.add_tx(make_tx(0xc0, 8, 55.0, Some(5.0), &tokens, &targets, &tags));

    let group = aggr.groups().values().next().unwrap();
    let detector = AttackDetector::new(1.0, 10);
    let verdict = detector.analyze_group(group).expect("should detect attack");
    assert!(matches!(verdict.attack_type, Some(AttackType::Sandwich { .. })));
}

#[test]
fn state_impact_deflationary_multi_victims() {
    let mut aggr = TxAggregator::new();
    let tokens = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tags = vec!["swap-v3".to_string()];

    // victims only for impact evaluation
    for i in 0..5u8 {
        aggr.add_tx(make_tx(0xd0 + i, (i as u64) + 1, 10.0, Some(1.0), &tokens, &targets, &tags));
    }

    let group = aggr.groups().values().next().unwrap();

    let victims: Vec<VictimInput> = group
        .txs
        .iter()
        .map(|t| VictimInput {
            tx_hash: t.tx_hash,
            amount_in: 100.0,
            amount_out_min: 97.0,
            token_behavior_unknown: true,
            flash_loan_amount: None,
        })
        .collect();

    let params = ImpactModelParams {
        curve_model: Arc::new(UniswapV3Curve::default()),
        lightweight_simulation: true,
        ..Default::default()
    };
    let mut evaluator = StateImpactEvaluator::new(params);
    let snapshot = default_snapshot();
    let impact = ImpactModel::evaluate_group(&mut evaluator, group, &victims, &snapshot);
    assert_eq!(impact.victims.len(), 5);
    assert!(impact.expected_profit_backrun > 0.0);
    assert_eq!(impact.impact_certainty, 0.61);
}

