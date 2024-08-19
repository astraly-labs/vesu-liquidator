pub mod config;
pub mod oracle;
pub mod services;
pub mod types;
pub mod utils;

use std::env;
use url::Url;

use anyhow::Result;

use tokio::sync::mpsc;

use crate::{
    config::PUBLIC_MAINNET_RPC, services::indexer::IndexerService,
    services::monitoring::MonitoringService, types::position::Position,
};

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url: Url = PUBLIC_MAINNET_RPC.parse()?;
    let pragma_api_key: String = env::var("PRAGMA_API_KEY")?;
    let apibara_key: String = env::var("APIBARA_API_KEY")?;

    // Channel for positions communication between services
    let (positions_sender, position_receiver) = mpsc::channel::<Position>(1024);
    let indexer_service = IndexerService::new(apibara_key, positions_sender);
    let monitoring_service = MonitoringService::new(rpc_url, pragma_api_key, position_receiver);

    // Index the available positions and sends them to the monitoring service
    let indexer_handle = tokio::spawn(async move {
        indexer_service.start().await;
    });

    // Monitors the positions health & liquidate if worthy
    let monitoring_handle = tokio::spawn(async move {
        monitoring_service.start().await;
    });

    tokio::try_join!(indexer_handle, monitoring_handle)?;

    Ok(())
}
