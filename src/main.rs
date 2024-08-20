pub mod cli;
pub mod config;
pub mod display;
pub mod oracle;
pub mod services;
pub mod types;
pub mod utils;

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use cli::RunCmd;
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
use tokio::sync::mpsc;
use types::account::StarknetAccountBuilder;

use crate::{
    services::indexer::IndexerService, services::monitoring::MonitoringService,
    types::position::Position,
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let mut run_cmd: RunCmd = RunCmd::parse();
    run_cmd.validate()?;

    display::print_app_title();

    let rpc_client = Arc::new(JsonRpcClient::new(HttpTransport::new(
        run_cmd.rpc_url.clone(),
    )));
    let account = StarknetAccountBuilder::from_cli(Arc::clone(&rpc_client), run_cmd.clone())?;

    let (positions_sender, position_receiver) = mpsc::channel::<Position>(1024);
    let indexer_service = IndexerService::new(
        Arc::clone(&rpc_client),
        run_cmd.apibara_api_key.unwrap(),
        positions_sender,
        run_cmd.starting_block,
    );
    let monitoring_service = MonitoringService::new(
        Arc::clone(&rpc_client),
        account,
        run_cmd.pragma_api_base_url.to_string(),
        run_cmd.pragma_api_key.unwrap(),
        position_receiver,
    );

    println!("🧩 Starting the indexer service...");
    let indexer_handle = tokio::spawn(async move {
        indexer_service
            .start()
            .await
            .context("😱 Indexer service failed!")
    });

    println!("⏳ Waiting a few moment for the indexer to fetch positions...");
    tokio::time::sleep(Duration::from_secs(15)).await;

    println!("\n🧩 Starting the monitoring service...");
    let monitoring_handle = tokio::spawn(async move {
        monitoring_service
            .start()
            .await
            .context("😱 Monitoring service error")
    });

    // Wait for both tasks to complete, and handle any errors
    let (indexer_result, monitoring_result) = tokio::try_join!(indexer_handle, monitoring_handle)?;

    // Handle results from both services
    indexer_result?;
    monitoring_result?;
    Ok(())
}
