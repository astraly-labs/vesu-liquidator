use bigdecimal::{num_bigint::BigInt, BigDecimal};

pub mod conversions;

pub fn apply_overhead(num: BigDecimal) -> BigDecimal {
    // we apply overhead of 2% as in vesu frontend
    let overhead_to_apply = BigDecimal::new(BigInt::from(102), 2);
    num * overhead_to_apply
}
