pub mod indexer;
pub mod monitoring;

use std::{sync::Arc, time::Duration};
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
    types::{account::StarknetAccount, position::Position},
};

/// Starts all the services needed by the Liquidator Bot.
/// This include:
/// - the indexer service, that indexes blocks & send positions,
/// - the monitoring service, that monitors & liquidates positions.
pub async fn start_liquidator_services(
    config: Config,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    account: StarknetAccount,
    run_cmd: RunCmd,
) -> Result<()> {
    let (positions_sender, position_receiver) = mpsc::channel::<Position>(1024);

    println!("🧩 Starting the indexer service...");
    let indexer_handle = start_indexer_service(
        config.clone(),
        rpc_client.clone(),
        positions_sender,
        run_cmd.starting_block,
        run_cmd.apibara_api_key.unwrap(),
    );

    println!("⏳ Waiting a few moment for the indexer to fetch positions...");
    tokio::time::sleep(Duration::from_secs(15)).await;

    println!("\n🧩 Starting the monitoring service...");
    let monitoring_handle = start_monitoring_service(
        config.clone(),
        rpc_client.clone(),
        account,
        run_cmd.pragma_api_base_url,
        run_cmd.pragma_api_key.unwrap(),
        position_receiver,
    );

    // Wait for both tasks to complete, and handle any errors
    let (indexer_result, monitoring_result) = tokio::try_join!(indexer_handle, monitoring_handle)?;

    // Handle results from both services
    indexer_result?;
    monitoring_result?;
    Ok(())
}

/// Starts the indexer service.
fn start_indexer_service(
    config: Config,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    positions_sender: Sender<Position>,
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

/// Starts the monitoring service.
fn start_monitoring_service(
    config: Config,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    account: StarknetAccount,
    pragma_api_base_url: Url,
    pragma_api_key: String,
    position_receiver: Receiver<Position>,
) -> JoinHandle<Result<()>> {
    let monitoring_service = MonitoringService::new(
        config,
        rpc_client,
        account,
        pragma_api_base_url,
        pragma_api_key,
        position_receiver,
    );

    tokio::spawn(async move {
        monitoring_service
            .start()
            .await
            .context("😱 Monitoring service error")
    })
}
