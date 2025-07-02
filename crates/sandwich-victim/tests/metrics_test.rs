use sandwich_victim::core::metrics::constant_product_input;
use ethereum_types::U256;

#[test]
fn constant_product_input_invalid_output() {
    let reserve_in = U256::from(100u64);
    let reserve_out = U256::from(50u64);
    let amount_out = U256::from(60u64);
    assert!(constant_product_input(amount_out, reserve_in, reserve_out).is_none());
}
