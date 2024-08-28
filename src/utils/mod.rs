use bigdecimal::{num_bigint::BigInt, BigDecimal};

pub mod conversions;
pub mod constants;

/// Apply a small overhead of 2% to the provided number.
pub fn apply_overhead(num: BigDecimal) -> BigDecimal {
    let overhead_to_apply = BigDecimal::new(BigInt::from(102), 2);
    num * overhead_to_apply
}

pub fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(false)
        .with_target(false)
        .init();
}
