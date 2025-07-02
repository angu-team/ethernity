use ethereum_types::U256;

pub trait U256Ext {
    fn to_f64_lossy(&self) -> f64;
}

impl U256Ext for U256 {
    fn to_f64_lossy(&self) -> f64 {
        let mut bytes = [0u8; 32];
        self.to_big_endian(&mut bytes);
        let mut result = 0f64;
        for &b in &bytes {
            result = result * 256f64 + b as f64;
        }
        result
    }
}

pub fn constant_product_output(amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
    if amount_in.is_zero() {
        return U256::zero();
    }
    let numerator = amount_in * reserve_out;
    numerator / (reserve_in + amount_in)
}

pub fn simulate_sandwich_profit(amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
    let front = amount_in / U256::from(10u64);
    let out_front = constant_product_output(front, reserve_in, reserve_out);
    let res_in_after_front = reserve_in + front;
    let res_out_after_front = reserve_out - out_front;
    let _victim_out = constant_product_output(amount_in, res_in_after_front, res_out_after_front);
    let res_in_after_victim = res_in_after_front + amount_in;
    let res_out_after_victim = res_out_after_front - _victim_out;
    let back_out = constant_product_output(out_front, res_out_after_victim, res_in_after_victim);
    if back_out > front { back_out - front } else { U256::zero() }
}

pub fn constant_product_input(
    amount_out: U256,
    reserve_in: U256,
    reserve_out: U256,
) -> Option<U256> {
    if amount_out >= reserve_out {
        return None;
    }
    let denominator = reserve_out - amount_out;
    if denominator.is_zero() {
        return None;
    }
    let numerator = reserve_in * amount_out;
    Some(numerator / denominator + U256::one())
}


