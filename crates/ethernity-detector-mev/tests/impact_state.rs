use std::collections::HashMap;
use std::sync::Arc;

use ethernity_detector_mev::{
    AnnotatedTx, ConstantProductCurve, ImpactModel, ImpactModelParams, PoolType,
    SlippageHistory, StateImpactEvaluator, StateSnapshot, TxAggregator, TxGroup,
    VictimInput, UniswapV3Curve, SnapshotEvent, CurveModel,
};
use ethereum_types::{Address, H256};
use tokio::sync::mpsc;

fn make_group(tags: Vec<String>) -> (TxAggregator, H256) {
    let mut aggr = TxAggregator::new();
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tx = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x10),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags,
        first_seen: 1,
        gas_price: 10.0,
        max_priority_fee_per_gas: None,
        confidence: 0.9,
    };
    let key = aggr.add_tx(tx).unwrap();
    (aggr, key)
}

fn group_from_tag(tag: &str) -> TxGroup {
    let token_paths = vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)];
    let targets = vec![Address::repeat_byte(0xaa)];
    let tx = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x10),
        token_paths: token_paths.clone(),
        targets: targets.clone(),
        tags: vec![tag.to_string()],
        first_seen: 1,
        gas_price: 10.0,
        max_priority_fee_per_gas: None,
        confidence: 0.9,
    };
    TxGroup {
        group_key: H256::repeat_byte(0x11),
        token_paths,
        targets,
        txs: vec![tx],
        block_number: None,
        direction_signature: String::new(),
        ordering_certainty_score: 1.0,
        reorderable: false,
        contaminated: false,
        window_start: 0,
    }
}

fn default_snapshot() -> StateSnapshot {
    StateSnapshot {
        reserve_in: 1000.0,
        reserve_out: 1000.0,
        sqrt_price_x96: Some(2_f64.powi(96)),
        liquidity: Some(1.0),
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    }
}

#[test]
fn constant_product_curve_calculation() {
    let curve = ConstantProductCurve::default();
    let snap = default_snapshot();
    let out = curve.expected_out(100.0, &snap);
    let expected = (100.0 * 997.0 * snap.reserve_out)
        / (snap.reserve_in * 1000.0 + 100.0 * 997.0);
    assert!((out - expected).abs() < 1e-6);
}

#[test]
fn uniswap_v3_curve_calculation() {
    let curve = UniswapV3Curve::default();
    let mut snap = default_snapshot();
    snap.sqrt_price_x96 = Some(2_f64.powi(96));
    let out = curve.expected_out(50.0, &snap);
    assert!((out - 50.0).abs() < 1e-6);
}

#[test]
fn apply_trade_updates_state() {
    let curve = ConstantProductCurve::default();
    let mut snap = default_snapshot();
    curve.apply_trade(100.0, &mut snap);
    let expected_out = (100.0 * 997.0 * 1000.0) / (1000.0 * 1000.0 + 100.0 * 997.0);
    assert!((snap.reserve_in - 1100.0).abs() < 1e-6);
    assert!((snap.reserve_out - (1000.0 - expected_out)).abs() < 1e-6);
}

#[test]
fn resolve_pool_type_variants() {
    let group_v2 = group_from_tag("swap-v2");
    assert_eq!(StateImpactEvaluator::resolve_pool_type(&group_v2), PoolType::V2);

    let group_v3 = group_from_tag("swap-v3");
    assert_eq!(StateImpactEvaluator::resolve_pool_type(&group_v3), PoolType::V3);

    let group_l = group_from_tag("lending");
    assert_eq!(StateImpactEvaluator::resolve_pool_type(&group_l), PoolType::Lending);

    let group_u = group_from_tag("token-move");
    assert_eq!(StateImpactEvaluator::resolve_pool_type(&group_u), PoolType::Unknown);
}

#[test]
fn slippage_tolerated_and_adjusted() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap();
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(ConstantProductCurve::default());
    let sc = params.slippage_curve;
    let mut ev = StateImpactEvaluator::new(params);

    let victims = vec![VictimInput {
        tx_hash: H256::repeat_byte(0x11),
        amount_in: 100.0,
        amount_out_min: 90.0,
        token_behavior_unknown: false,
        flash_loan_amount: None,
    }];
    let snap = default_snapshot();
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snap);
    let victim = &res.victims[0];
    let expected = (100.0 * 997.0 * 1000.0) / (1000.0 * 1000.0 + 100.0 * 997.0);
    let slippage = ((expected - 90.0) / expected) * 100.0;
    assert!((victim.slippage_tolerated - slippage).abs() < 1e-6);
    assert!((victim.slippage_baseline - sc).abs() < 1e-6);
    assert!((victim.slippage_adjusted - (slippage + sc) / 2.0).abs() < 1e-6);
}

#[test]
fn slippage_history_window_average() {
    let mut hist = SlippageHistory::new(2);
    hist.record(1.0);
    assert_eq!(hist.average(), 1.0);
    hist.record(3.0);
    assert_eq!(hist.average(), 2.0);
    hist.record(5.0);
    assert_eq!(hist.average(), 4.0);
}

#[test]
fn dynamic_vs_static_baseline() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap();
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(ConstantProductCurve::default());
    let sc = params.slippage_curve;
    let mut ev = StateImpactEvaluator::new(params);
    let victims = vec![VictimInput { tx_hash: H256::zero(), amount_in: 50.0, amount_out_min: 45.0, token_behavior_unknown: false, flash_loan_amount: None }];
    let snap = default_snapshot();
    let res1 = ImpactModel::evaluate_group(&mut ev, group, &victims, &snap);
    let baseline1 = res1.victims[0].slippage_baseline;
    let res2 = ImpactModel::evaluate_group(&mut ev, group, &victims, &snap);
    let baseline2 = res2.victims[0].slippage_baseline;
    assert_eq!(baseline1, sc);
    assert_ne!(baseline2, sc);
}

#[test]
fn state_confidence_and_impact_certainty() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap();

    let victims = vec![VictimInput { tx_hash: H256::zero(), amount_in: 50.0, amount_out_min: 40.0, token_behavior_unknown: true, flash_loan_amount: None }];
    let mut snap = default_snapshot();
    snap.state_lag_blocks = 3;
    snap.reorg_risk_level = "high".into();
    let res = StateImpactEvaluator::evaluate(group, &victims, &snap);
    assert!(res.state_confidence < 1.0);
    assert_eq!(res.impact_certainty, 0.61);
}

#[test]
fn opportunity_score_penalty_with_convexity() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap();
    let params = ImpactModelParams { curve_model: Arc::new(ConstantProductCurve::default()), ..Default::default() };
    let mut ev = StateImpactEvaluator::new(params);
    let victims = vec![
        VictimInput { tx_hash: H256::repeat_byte(0x01), amount_in: 100.0, amount_out_min: 90.0, token_behavior_unknown: false, flash_loan_amount: None },
        VictimInput { tx_hash: H256::repeat_byte(0x02), amount_in: 50.0, amount_out_min: 40.0, token_behavior_unknown: false, flash_loan_amount: None },
    ];
    let snap = default_snapshot();
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snap);
    assert!(res.opportunity_score < 0.9);
}

#[test]
fn multiple_victims_lightweight_simulation() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap();
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(ConstantProductCurve::default());
    params.lightweight_simulation = true;
    let mut ev = StateImpactEvaluator::new(params);
    let victims = vec![
        VictimInput { tx_hash: H256::repeat_byte(0x01), amount_in: 100.0, amount_out_min: 90.0, token_behavior_unknown: false, flash_loan_amount: None },
        VictimInput { tx_hash: H256::repeat_byte(0x02), amount_in: 100.0, amount_out_min: 90.0, token_behavior_unknown: false, flash_loan_amount: None },
    ];
    let snap = default_snapshot();
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snap);
    assert!(res.victims[1].expected_amount_out < res.victims[0].expected_amount_out);
    let profit = (res.victims[0].expected_amount_out - 90.0)
        + (res.victims[1].expected_amount_out - 90.0);
    assert!((res.expected_profit_backrun - profit).abs() < 1e-6);
}

#[tokio::test]
async fn process_stream_pipeline() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap().clone();
    let snapshot = default_snapshot();
    let mut map = HashMap::new();
    map.insert(Address::repeat_byte(0xaa), snapshot);
    let (tx_in, rx_in) = mpsc::channel(1);
    let (tx_out, mut rx_out) = mpsc::channel(1);
    tokio::spawn(async move { StateImpactEvaluator::process_stream(rx_in, tx_out).await; });
    tx_in
        .send(SnapshotEvent { group: group.clone(), snapshots: map })
        .await
        .unwrap();
    drop(tx_in);
    let ev = rx_out.recv().await.expect("no impact event");
    assert_eq!(ev.group.group_key, group.group_key);
}

#[test]
fn flash_loan_impact_detection() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap();
    let params = ImpactModelParams {
        curve_model: Arc::new(ConstantProductCurve::default()),
        lightweight_simulation: true,
        ..Default::default()
    };
    let mut ev = StateImpactEvaluator::new(params);
    let victims = vec![VictimInput {
        tx_hash: H256::repeat_byte(0x21),
        amount_in: 100.0,
        amount_out_min: 90.0,
        token_behavior_unknown: false,
        flash_loan_amount: Some(10_000_000.0),
    }];
    let snapshot = StateSnapshot {
        reserve_in: 1_000_000.0,
        reserve_out: 1_000_000.0,
        sqrt_price_x96: None,
        liquidity: None,
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    };
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snapshot);
    let denom = 1_000_000.0 * 1000.0 + 10_000_000.0 * 997.0;
    let expected = (10_000_000.0 * 997.0 * 1_000_000.0) / denom;
    let profit = expected - 90.0;
    assert!((res.expected_profit_backrun - profit).abs() < 1e-6);
    assert!((res.opportunity_score - 0.95).abs() < 1e-6);
}

#[test]
fn zero_liquidity_pool_returns_zero() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap();
    let victims = vec![VictimInput {
        tx_hash: H256::zero(),
        amount_in: 100.0,
        amount_out_min: 0.0,
        token_behavior_unknown: false,
        flash_loan_amount: None,
    }];
    let snapshot = StateSnapshot {
        reserve_in: 0.0,
        reserve_out: 0.0,
        sqrt_price_x96: None,
        liquidity: None,
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    };
    let mut ev = StateImpactEvaluator::default();
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snapshot);
    assert_eq!(res.victims[0].expected_amount_out, 0.0);
}

#[test]
fn high_volatility_penalizes_confidence() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap();
    let victims = vec![VictimInput { tx_hash: H256::zero(), amount_in: 100.0, amount_out_min: 90.0, token_behavior_unknown: false, flash_loan_amount: None }];
    let mut snap = default_snapshot();
    snap.volatility_flag = true;
    let res = StateImpactEvaluator::evaluate(group, &victims, &snap);
    assert!(res.state_confidence < 1.0);
}

#[test]
fn dynamic_fee_impact() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap();
    let curve = ConstantProductCurve { fee_rate: 0.005 };
    let params = ImpactModelParams { curve_model: Arc::new(curve), ..Default::default() };
    let mut ev = StateImpactEvaluator::new(params);
    let victims = vec![VictimInput { tx_hash: H256::zero(), amount_in: 100.0, amount_out_min: 95.0, token_behavior_unknown: false, flash_loan_amount: None }];
    let snap = default_snapshot();
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snap);
    let fee_mul = 1.0 - 0.005;
    let denom = snap.reserve_in + 100.0 * fee_mul;
    let expected = 100.0 * fee_mul * snap.reserve_out / denom;
    assert!((res.victims[0].expected_amount_out - expected).abs() < 1e-6);
}

#[tokio::test]
async fn process_stream_multiple_pools() {
    let (aggr, key) = make_group(vec!["swap-v2".into()]);
    let group = aggr.groups().get(&key).unwrap().clone();
    let snapshot = default_snapshot();
    let mut map = HashMap::new();
    map.insert(Address::repeat_byte(0xaa), snapshot.clone());
    map.insert(Address::repeat_byte(0xbb), snapshot);
    let (tx_in, rx_in) = mpsc::channel(2);
    let (tx_out, mut rx_out) = mpsc::channel(2);
    tokio::spawn(async move { StateImpactEvaluator::process_stream(rx_in, tx_out).await; });
    tx_in.send(SnapshotEvent { group: group.clone(), snapshots: map }).await.unwrap();
    drop(tx_in);
    let ev1 = rx_out.recv().await.expect("ev1");
    let ev2 = rx_out.recv().await.expect("ev2");
    assert_eq!(ev1.group.group_key, group.group_key);
    assert_eq!(ev2.group.group_key, group.group_key);
}
