use std::env;
use url::Url;

use anyhow::Result;

use tokio::sync::mpsc;

use vesu_liquidator::{
    config::PUBLIC_MAINNET_RPC, indexer::IndexerService, monitoring::MonitoringService,
    types::Position,
};

#[tokio::main]
async fn main() -> Result<()> {
    let (position_sender, position_receiver) = mpsc::channel::<Position>(1024);

    let rpc_url: Url = PUBLIC_MAINNET_RPC.parse()?;
    let pragma_api_key: String = env::var("PRAGMA_API_KEY")?;
    let monitoring_service = MonitoringService::new(rpc_url, pragma_api_key, position_receiver);

    let apibara_key: String = env::var("APIBARA_API_KEY")?;
    let indexer = IndexerService::new(apibara_key);

    // Spawn indexer.start in a separate task
    let indexer_handle = tokio::spawn(async move {
        indexer.start(position_sender).await;
    });

    // Spawn monitoring service in a separate task
    let monitoring_handle = tokio::spawn(async move {
        monitoring_service.start().await;
    });

    // Wait for both tasks to complete (they shouldn't under normal circumstances)
    tokio::try_join!(indexer_handle, monitoring_handle)?;

    Ok(())
}
