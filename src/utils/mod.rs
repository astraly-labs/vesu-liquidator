pub mod constants;
pub mod conversions;
pub mod ekubo;
pub mod services;

use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::bail;
use starknet::{
    core::types::{ExecutionResult, Felt, StarknetError},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, ProviderError},
};

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
    rpc_client: &Arc<JsonRpcClient<HttpTransport>>,
    tx_hash: Felt,
) -> anyhow::Result<()> {
    const WAIT_FOR_TX_TIMEOUT: Duration = Duration::from_secs(15);
    const CHECK_INTERVAL: Duration = Duration::from_secs(1);

    let start = SystemTime::now();

    loop {
        if start.elapsed().unwrap() >= WAIT_FOR_TX_TIMEOUT {
            bail!("Timeout while waiting for transaction {tx_hash:#064x}");
        }

        match rpc_client.get_transaction_receipt(tx_hash).await {
            Ok(tx) => match tx.receipt.execution_result() {
                ExecutionResult::Succeeded => {
                    return Ok(());
                }
                ExecutionResult::Reverted { reason } => {
                    bail!(format!(
                        "Transaction {tx_hash:#064x} has been rejected/reverted: {reason}"
                    ));
                }
            },
            Err(ProviderError::StarknetError(StarknetError::TransactionHashNotFound)) => {
                tracing::debug!("Waiting for transaction {tx_hash:#064x} to show up");
                tokio::time::sleep(CHECK_INTERVAL).await;
            }
            Err(err) => {
                bail!("Error while waiting for transaction {tx_hash:#064x}: {err:?}");
            }
        }
    }
}
