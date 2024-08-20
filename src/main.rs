pub mod config;
pub mod oracle;
pub mod services;
pub mod types;
pub mod utils;

use std::{env, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use starknet::{
    core::types::Felt,
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
};
use tokio::sync::mpsc;
use url::Url;

use crate::{
    config::PUBLIC_MAINNET_RPC, services::indexer::IndexerService,
    services::monitoring::MonitoringService, types::account::StarknetAccount,
    types::position::Position,
};

pub const CHANNEL_POSITIONS_SIZE: usize = 1024;

// TODO: Should be CLI args + Handle keystores
pub const ACCOUNT_ADDRESS: &str =
    "0x042f09c629f993Bd4ce1f6524C24aeD223c7c4b967D732A9A4674Cf07088cc6c";
pub const PRIVATE_KEY: &str = "0x01a76e1a8d42bf894161b62fbbc5406e2319dedf39214a98e12df67dd613942d";

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    let pragma_api_key: String = env::var("PRAGMA_API_KEY")?;
    let apibara_key: String = env::var("APIBARA_API_KEY")?;

    let rpc_url: Url = PUBLIC_MAINNET_RPC.parse()?;
    let rpc_client = Arc::new(JsonRpcClient::new(HttpTransport::new(rpc_url)));

    // TODO: Allow from keystore + CLI args
    let account = StarknetAccount::from_secret(
        rpc_client.clone(),
        Felt::from_hex(ACCOUNT_ADDRESS).unwrap(),
        Felt::from_hex(PRIVATE_KEY).unwrap(),
    );

    let (positions_sender, position_receiver) = mpsc::channel::<Position>(CHANNEL_POSITIONS_SIZE);
    let indexer_service =
        IndexerService::new(Arc::clone(&rpc_client), apibara_key, positions_sender);
    let monitoring_service = MonitoringService::new(
        Arc::clone(&rpc_client),
        account,
        pragma_api_key,
        position_receiver,
    );

    let indexer_handle = tokio::spawn(async move {
        indexer_service
            .start()
            .await
            .context("😱 Indexer service error")
    });

    println!("⏳ Waiting a few moments for the indexer to catch up...");
    tokio::time::sleep(Duration::from_secs(15)).await;

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
