pub mod cli;
pub mod config;
pub mod services;
pub mod storages;
pub mod types;
pub mod utils;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use starknet::{
    core::types::Felt,
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
};

use cli::{NetworkName, RunCmd};
use config::Config;
use services::start_all_services;
use types::account::StarknetAccount;
use utils::setup_tracing;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    setup_tracing();

    let mut run_cmd = RunCmd::parse();
    run_cmd.validate()?;

    print_app_title(
        run_cmd.account_params.account_address,
        run_cmd.network,
        run_cmd.starting_block,
    );

    let rpc_url = run_cmd.rpc_url.clone();
    let rpc_client = Arc::new(JsonRpcClient::new(HttpTransport::new(rpc_url)));
    let account = StarknetAccount::from_cli(rpc_client.clone(), run_cmd.clone())?;

    let config = Config::from_cli(&run_cmd)?;
    start_all_services(config, rpc_client, account, run_cmd).await
}

/// Prints information about the bot parameters.
fn print_app_title(account_address: Felt, network: NetworkName, starting_block: u64) {
    println!("\n
██╗   ██╗███████╗███████╗██╗   ██╗    ██╗     ██╗ ██████╗ ██╗   ██╗██╗██████╗  █████╗ ████████╗ ██████╗ ██████╗ 
██║   ██║██╔════╝██╔════╝██║   ██║    ██║     ██║██╔═══██╗██║   ██║██║██╔══██╗██╔══██╗╚══██╔══╝██╔═══██╗██╔══██╗
██║   ██║█████╗  ███████╗██║   ██║    ██║     ██║██║   ██║██║   ██║██║██║  ██║███████║   ██║   ██║   ██║██████╔╝
╚██╗ ██╔╝██╔══╝  ╚════██║██║   ██║    ██║     ██║██║▄▄ ██║██║   ██║██║██║  ██║██╔══██║   ██║   ██║   ██║██╔══██╗
 ╚████╔╝ ███████╗███████║╚██████╔╝    ███████╗██║╚██████╔╝╚██████╔╝██║██████╔╝██║  ██║   ██║   ╚██████╔╝██║  ██║
  ╚═══╝  ╚══════╝╚══════╝ ╚═════╝     ╚══════╝╚═╝ ╚══▀▀═╝  ╚═════╝ ╚═╝╚═════╝ ╚═╝  ╚═╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝

  🤖 Liquidator 👉 0x{:x}
  🎯 On {}
  🥡 Starting from block {}
    \n", account_address, network, starting_block);
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf, sync::Arc};

    use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
    use tokio::sync::mpsc;
    use url::Url;

    use crate::{cli::NetworkName, config::Config, services::indexer::IndexerService, types::position::Position};

    #[test]
    fn e2e_testing() {
        // create indexer
        let config = Config::new(NetworkName::Mainnet, &PathBuf::from("./config.yaml")).unwrap();
        let rpc_url = Url::parse("https://starknet-mainnet.public.blastapi.io").unwrap();
        let rpc_client = Arc::new(JsonRpcClient::new(HttpTransport::new(rpc_url)));
        let (positions_sender, position_receiver) = mpsc::channel::<Position>(1024);

        let indexer = IndexerService::new(
            config.clone(),
            rpc_client.clone(),
            env::var("APIBARA_KEY").unwrap(),
            positions_sender,
            600000,
        );
        // assert that a position is retrieved
        // create monitoring
        // 

        todo!()
    }
}
