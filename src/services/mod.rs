pub mod indexer;
pub mod monitoring;
pub mod oracle;

use std::{cmp, sync::Arc};

use anyhow::Result;
use starknet::providers::{JsonRpcClient, jsonrpc::HttpTransport};
use tokio::sync::mpsc::unbounded_channel;

use oracle::{LatestOraclePrices, OracleService};

use crate::{
    cli::RunCmd,
    config::Config,
    services::{indexer::IndexerService, monitoring::MonitoringService},
    storages::{Storage, json::JsonStorage},
    types::{account::StarknetAccount, position::Position},
    utils::services::{Service, ServiceGroup},
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
    let (positions_sender, position_receiver) = unbounded_channel::<(u64, Position)>();

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

    let starting_block = cmp::max(run_cmd.starting_block, last_block_indexed);
    println!("  🥡 Starting from block {}\n\n", starting_block);

    let indexer_service = IndexerService::new(
        config.clone(),
        run_cmd.apibara_api_key.unwrap(),
        positions_sender,
        starting_block,
    );
    let latest_oracle_prices = LatestOraclePrices::from_config(&config);
    let oracle_service = OracleService::new(
        config.pragma_oracle_address,
        rpc_client.clone(),
        latest_oracle_prices.clone(),
    );
    let monitoring_service = MonitoringService::new(
        config,
        rpc_client,
        account,
        position_receiver,
        latest_oracle_prices,
        Box::new(storage),
    );

    ServiceGroup::default()
        .with(indexer_service)
        .with(oracle_service)
        .with(monitoring_service)
        .start_and_drive_to_end()
        .await?;

    Ok(())
}
