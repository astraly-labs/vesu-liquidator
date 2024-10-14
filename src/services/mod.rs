pub mod indexer;
pub mod monitoring;
pub mod oracle;

use bigdecimal::BigDecimal;
use oracle::{LatestOraclePrices, OracleMode, OracleService, OracleServiceMode};
use std::{cmp, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    cli::{NetworkName, RunCmd},
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
    let mut storage = JsonStorage::new(
        run_cmd
            .storage_path
            .unwrap_or_default()
            .as_path()
            .to_str()
            .unwrap_or_default(),
    );
    let (last_block_indexed, _) = storage.load().await?;

    // TODO: Add force start from staring block in cli
    let starting_block = cmp::max(run_cmd.starting_block, last_block_indexed);
    println!("  🥡 Starting from block {}\n\n", starting_block);

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
    let oracle_handle: JoinHandle<std::prelude::v1::Result<(), anyhow::Error>> =
        start_oracle_service(
            run_cmd.pragma_api_base,
            run_cmd.pragma_api_key.unwrap(),
            latest_oracle_prices.clone(),
            run_cmd.network,
            match run_cmd.oracle_mode {
                OracleMode::Http => {
                    OracleServiceMode::Http(Duration::from_secs(run_cmd.prices_update_interval))
                }
                OracleMode::WebSocket => OracleServiceMode::WebSocket,
            },
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
        run_cmd.check_positions_interval,
        run_cmd.min_profit,
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
    pragma_api_base: String,
    pragma_api_key: String,
    latest_oracle_prices: LatestOraclePrices,
    network: NetworkName,
    oracle_mode: OracleServiceMode,
) -> JoinHandle<Result<()>> {
    let oracle_service = OracleService::new(
        pragma_api_base,
        pragma_api_key,
        latest_oracle_prices,
        network,
        oracle_mode,
    );

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
    check_positions_interval: u64,
    min_profit: BigDecimal,
) -> JoinHandle<Result<()>> {
    let monitoring_service = MonitoringService::new(
        config,
        rpc_client,
        account,
        position_receiver,
        latest_oracle_prices,
        storage,
        check_positions_interval,
        min_profit,
    );

    tokio::spawn(async move {
        monitoring_service
            .start()
            .await
            .context("😱 Monitoring service error")
    })
}
