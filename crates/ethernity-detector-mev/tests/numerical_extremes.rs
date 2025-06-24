use ethernity_detector_mev::{
    AnnotatedTx, TxAggregator, VictimInput, StateSnapshot,
    StateImpactEvaluator, ConstantProductCurve, UniswapV3Curve, ImpactModel,
    ImpactModelParams, CurveModel
};
use ethereum_types::{Address, H256};
use std::sync::Arc;

fn make_group(tag: &str) -> (TxAggregator, H256) {
    let mut aggr = TxAggregator::new();
    let tx = AnnotatedTx {
        tx_hash: H256::repeat_byte(0x10),
        token_paths: vec![Address::repeat_byte(0x01), Address::repeat_byte(0x02)],
        targets: vec![Address::repeat_byte(0xaa)],
        tags: vec![tag.to_string()],
        first_seen: 1,
        gas_price: 10.0,
        max_priority_fee_per_gas: None,
        confidence: 1.0,
    };
    let key = aggr.add_tx(tx).unwrap();
    (aggr, key)
}

#[test]
fn constant_product_precision_limits() {
    let (aggr, key) = make_group("swap-v2");
    let group = aggr.groups().get(&key).unwrap();
    let victims = vec![VictimInput {
        tx_hash: H256::zero(),
        amount_in: f64::MIN_POSITIVE,
        amount_out_min: 0.0,
        token_behavior_unknown: false,
        flash_loan_amount: None,
    }];
    let snapshot = StateSnapshot {
        reserve_in: f64::MIN_POSITIVE,
        reserve_out: f64::MIN_POSITIVE,
        sqrt_price_x96: None,
        liquidity: None,
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    };
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(ConstantProductCurve::default());
    let mut ev = StateImpactEvaluator::new(params);
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snapshot);
    let out = res.victims[0].expected_amount_out;
    assert!(out.is_finite());
    assert!(out >= 0.0);
}

#[test]
fn multiplication_overflow_v2() {
    let (aggr, key) = make_group("swap-v2");
    let group = aggr.groups().get(&key).unwrap();
    let victims = vec![VictimInput {
        tx_hash: H256::zero(),
        amount_in: f64::MAX / 2.0,
        amount_out_min: 0.0,
        token_behavior_unknown: false,
        flash_loan_amount: None,
    }];
    let snapshot = StateSnapshot {
        reserve_in: f64::MAX / 2.0,
        reserve_out: f64::MAX / 2.0,
        sqrt_price_x96: None,
        liquidity: None,
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    };
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(ConstantProductCurve::default());
    let mut ev = StateImpactEvaluator::new(params);
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snapshot);
    assert_eq!(res.victims[0].expected_amount_out, 0.0);
}

#[test]
fn slippage_with_tiny_reserves() {
    let (aggr, key) = make_group("swap-v2");
    let group = aggr.groups().get(&key).unwrap();
    let victims = vec![VictimInput {
        tx_hash: H256::zero(),
        amount_in: 1.0,
        amount_out_min: 0.5,
        token_behavior_unknown: false,
        flash_loan_amount: None,
    }];
    let snapshot = StateSnapshot {
        reserve_in: 1e-12,
        reserve_out: 1e-12,
        sqrt_price_x96: None,
        liquidity: None,
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    };
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(ConstantProductCurve::default());
    let mut ev = StateImpactEvaluator::new(params);
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snapshot);
    let slip = res.victims[0].slippage_tolerated;
    assert!(slip.is_finite());
    assert!(slip <= 0.0);
}

#[test]
fn precision_multiple_victims_high_precision() {
    let (aggr, key) = make_group("swap-v2");
    let group = aggr.groups().get(&key).unwrap();
    let victims = vec![
        VictimInput { tx_hash: H256::repeat_byte(0x01), amount_in: 1.1, amount_out_min: 1.0, token_behavior_unknown: false, flash_loan_amount: None },
        VictimInput { tx_hash: H256::repeat_byte(0x02), amount_in: 2.2, amount_out_min: 2.0, token_behavior_unknown: false, flash_loan_amount: None },
        VictimInput { tx_hash: H256::repeat_byte(0x03), amount_in: 3.3, amount_out_min: 3.0, token_behavior_unknown: false, flash_loan_amount: None },
    ];
    let snapshot = StateSnapshot {
        reserve_in: 1000.0,
        reserve_out: 1000.0,
        sqrt_price_x96: None,
        liquidity: None,
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    };
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(ConstantProductCurve::default());
    params.lightweight_simulation = true;
    let mut ev = StateImpactEvaluator::new(params);
    let result = ImpactModel::evaluate_group(&mut ev, group, &victims, &snapshot);
    let mut expected_profit = 0.0;
    let curve = ConstantProductCurve::default();
    let mut snap = snapshot.clone();
    for (i, v) in victims.iter().enumerate() {
        let out = curve.expected_out(v.amount_in, &snap);
        expected_profit += out - v.amount_out_min;
        curve.apply_trade(v.amount_in, &mut snap);
        assert!((result.victims[i].expected_amount_out - out).abs() < 1e-6);
    }
    assert!((result.expected_profit_backrun - expected_profit).abs() < 1e-6);
}

#[test]
fn uniswap_v3_extremely_volatile_price() {
    let (aggr, key) = make_group("swap-v3");
    let group = aggr.groups().get(&key).unwrap();
    let victims = vec![VictimInput { tx_hash: H256::zero(), amount_in: 1.0, amount_out_min: 0.0, token_behavior_unknown: false, flash_loan_amount: None }];
    let snap1 = StateSnapshot { reserve_in: 0.0, reserve_out: 0.0, sqrt_price_x96: Some(1e5), liquidity: None, state_lag_blocks: 0, reorg_risk_level: "low".into(), volatility_flag: true };
    let snap2 = StateSnapshot { reserve_in: 0.0, reserve_out: 0.0, sqrt_price_x96: Some(1e160), liquidity: None, state_lag_blocks: 0, reorg_risk_level: "low".into(), volatility_flag: true };
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(UniswapV3Curve);
    let mut ev = StateImpactEvaluator::new(params.clone());
    let r1 = ImpactModel::evaluate_group(&mut ev, group, &victims, &snap1);
    let o1 = r1.victims[0].expected_amount_out;
    let r2 = ImpactModel::evaluate_group(&mut ev, group, &victims, &snap2);
    let o2 = r2.victims[0].expected_amount_out;
    assert!(o1.is_finite() && o1 > 0.0);
    assert_eq!(o2, 0.0);
}
