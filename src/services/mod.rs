pub mod indexer;
pub mod monitoring;
pub mod oracle;

use oracle::{LatestOraclePrices, OracleService};
use std::{cmp, sync::Arc, time::Duration};
use url::Url;

use anyhow::{Context, Result};
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    cli::RunCmd,
    config::Config,
    services::{indexer::IndexerService, monitoring::MonitoringService},
    storages::{json::JsonStorage, Storage},
    types::{account::StarknetAccount, position::Position},
};

/// Starts all the services needed by the Liquidator Bot.
/// This include:
/// - the indexer service, that indexes blocks & send positions,
/// - the monitoring service, that monitors & liquidates positions.
pub async fn start_all_services(
    config: Config,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    account: StarknetAccount,
    run_cmd: RunCmd,
) -> Result<()> {
    let (positions_sender, position_receiver) = mpsc::channel::<(u64, Position)>(1024);

    // TODO: Add new methods of storage (s3, postgres, sqlite) and be able to define them in CLI
    let mut storage = JsonStorage::new("data.json");
    let (last_block_indexed, _) = storage.load().await?;

    // TODO: Add force start from staring block in cli
    let starting_block = cmp::max(run_cmd.starting_block, last_block_indexed);

    tracing::info!("🧩 Starting the indexer service...");
    let indexer_handle = start_indexer_service(
        config.clone(),
        rpc_client.clone(),
        positions_sender,
        starting_block,
        run_cmd.apibara_api_key.unwrap(),
    );

    let latest_oracle_prices = LatestOraclePrices::from_config(&config);
    tracing::info!("🧩 Starting the oracle service...");
    let oracle_handle = start_oracle_service(
        run_cmd.pragma_api_base_url,
        run_cmd.pragma_api_key.unwrap(),
        latest_oracle_prices.clone(),
    );

    tracing::info!("⏳ Waiting a few moment for the indexer to fetch positions...\n");
    tokio::time::sleep(Duration::from_secs(10)).await;

    tracing::info!("🧩 Starting the monitoring service...\n");
    let monitoring_handle = start_monitoring_service(
        config.clone(),
        rpc_client.clone(),
        account,
        position_receiver,
        latest_oracle_prices,
        Box::new(storage),
    );

    // Wait for tasks to complete, and handle any errors
    let (indexer_result, oracle_result, monitoring_result) =
        tokio::try_join!(indexer_handle, oracle_handle, monitoring_handle)?;

    // Handle results
    indexer_result?;
    oracle_result?;
    monitoring_result?;
    Ok(())
}

/// Starts the indexer service.
fn start_indexer_service(
    config: Config,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    positions_sender: Sender<(u64, Position)>,
    starting_block: u64,
    apibara_api_key: String,
) -> JoinHandle<Result<()>> {
    let indexer_service = IndexerService::new(
        config,
        rpc_client,
        apibara_api_key,
        positions_sender,
        starting_block,
    );

    tokio::spawn(async move {
        indexer_service
            .start()
            .await
            .context("😱 Indexer service failed!")
    })
}

/// Starts the oracle service.
fn start_oracle_service(
    pragma_api_base_url: Url,
    pragma_api_key: String,
    latest_oracle_prices: LatestOraclePrices,
) -> JoinHandle<Result<()>> {
    let oracle_service =
        OracleService::new(pragma_api_base_url, pragma_api_key, latest_oracle_prices);

    tokio::spawn(async move {
        oracle_service
            .start()
            .await
            .context("😱 Oracle service error")
    })
}

/// Starts the monitoring service.
fn start_monitoring_service(
    config: Config,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    account: StarknetAccount,
    position_receiver: Receiver<(u64, Position)>,
    latest_oracle_prices: LatestOraclePrices,
    storage: Box<dyn Storage>,
) -> JoinHandle<Result<()>> {
    let monitoring_service = MonitoringService::new(
        config,
        rpc_client,
        account,
        position_receiver,
        latest_oracle_prices,
        storage,
    );

    tokio::spawn(async move {
        monitoring_service
            .start()
            .await
            .context("😱 Monitoring service error")
    })
}
