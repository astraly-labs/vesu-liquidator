use bigdecimal::{num_bigint::BigInt, BigDecimal};

pub mod conversions;

pub fn apply_overhead(num: BigDecimal) -> BigDecimal {
    // we apply overhead of 2% as in vesu frontend
    let overhead_to_apply = BigDecimal::new(BigInt::from(102), 2);
    num * overhead_to_apply
}

pub fn setup_tracing() {
    tracing_subscriber::fmt()
        // enable everything
        .with_max_level(tracing::Level::INFO)
        .compact()
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(false)
        .with_target(false)
        .init();
}
