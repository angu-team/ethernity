use ethernity_detector_mev::{
    AnnotatedTx, TxAggregator, VictimInput, StateSnapshot,
    StateImpactEvaluator, ConstantProductCurve, UniswapV3Curve, ImpactModel,
    ImpactModelParams,
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
fn constant_product_low_liquidity() {
    let (aggr, key) = make_group("swap-v2");
    let group = aggr.groups().get(&key).unwrap();
    let victims = vec![VictimInput {
        tx_hash: H256::zero(),
        amount_in: 1.0,
        amount_out_min: 0.0,
        token_behavior_unknown: false,
        flash_loan_amount: None,
    }];
    let snapshot = StateSnapshot {
        reserve_in: 1e-18,
        reserve_out: 1e-18,
        sqrt_price_x96: None,
        liquidity: None,
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    };
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(ConstantProductCurve);
    let mut ev = StateImpactEvaluator::new(params);
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snapshot);
    let out = res.victims[0].expected_amount_out;
    assert!(out.is_finite());
    assert!(out >= 0.0);
}

#[test]
fn constant_product_high_liquidity() {
    let (aggr, key) = make_group("swap-v2");
    let group = aggr.groups().get(&key).unwrap();
    let victims = vec![VictimInput {
        tx_hash: H256::zero(),
        amount_in: 1e6,
        amount_out_min: 0.0,
        token_behavior_unknown: false,
        flash_loan_amount: None,
    }];
    let snapshot = StateSnapshot {
        reserve_in: 1e30,
        reserve_out: 1e30,
        sqrt_price_x96: None,
        liquidity: None,
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    };
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(ConstantProductCurve);
    let mut ev = StateImpactEvaluator::new(params);
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snapshot);
    let out = res.victims[0].expected_amount_out;
    assert!(out.is_finite());
    assert!(out >= 0.0);
}

#[test]
fn uniswap_v3_extreme_price() {
    let (aggr, key) = make_group("swap-v3");
    let group = aggr.groups().get(&key).unwrap();
    let victims = vec![VictimInput {
        tx_hash: H256::zero(),
        amount_in: 10.0,
        amount_out_min: 0.0,
        token_behavior_unknown: false,
        flash_loan_amount: None,
    }];
    let snapshot = StateSnapshot {
        reserve_in: 0.0,
        reserve_out: 0.0,
        sqrt_price_x96: Some(1e15),
        liquidity: None,
        state_lag_blocks: 0,
        reorg_risk_level: "low".into(),
        volatility_flag: false,
    };
    let mut params = ImpactModelParams::default();
    params.curve_model = Arc::new(UniswapV3Curve);
    let mut ev = StateImpactEvaluator::new(params);
    let res = ImpactModel::evaluate_group(&mut ev, group, &victims, &snapshot);
    let out = res.victims[0].expected_amount_out;
    assert!(out.is_finite());
    assert!(out >= 0.0);
}
