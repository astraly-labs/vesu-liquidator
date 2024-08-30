use std::{sync::Arc, time::Duration};

use bigdecimal::{num_bigint::BigInt, BigDecimal};
use constants::{INTERVAL_CHECK_TX_FINALITY, MAX_RETRIES_VERIFY_TX_FINALITY};
use starknet::{
    core::types::{Felt, TransactionFinalityStatus},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
};

pub mod constants;
pub mod conversions;
#[cfg(test)]
pub mod test_utils;

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

pub async fn wait_for_tx(
    tx_hash: Felt,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
) -> anyhow::Result<()> {
    let mut retries = 0;
    let duration_to_wait_between_polling = Duration::from_secs(INTERVAL_CHECK_TX_FINALITY);
    tokio::time::sleep(duration_to_wait_between_polling).await;

    loop {
        let response = rpc_client.get_transaction_receipt(tx_hash).await?;
        let status = response.receipt.finality_status();
        if *status != TransactionFinalityStatus::AcceptedOnL2 {
            retries += 1;
            if retries > MAX_RETRIES_VERIFY_TX_FINALITY {
                return Err(anyhow::anyhow!(
                    "Max retries exceeeded while waiting for tx {tx_hash} finality."
                ));
            }
            tokio::time::sleep(duration_to_wait_between_polling).await;
        } else {
            break;
        }
    }
    Ok(())
}
