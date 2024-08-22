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
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Display the thread ID an event was recorded on
        .with_thread_ids(true)
        // Don't display the event's target (module path)
        .with_target(false)
        // sets this to be the default, global collector for this application.
        .init();
}
