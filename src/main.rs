pub mod cli;
pub mod config;
pub mod services;
pub mod storages;
pub mod types;
pub mod utils;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use starknet_rust::{
    core::types::Felt,
    providers::{JsonRpcClient, jsonrpc::HttpTransport},
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

    print_app_title(run_cmd.account_params.account_address, run_cmd.network);

    let rpc_url = run_cmd.rpc_url.clone();
    let rpc_client = Arc::new(JsonRpcClient::new(HttpTransport::new(rpc_url)));
    let account = StarknetAccount::from_cli(rpc_client.clone(), run_cmd.clone())?;

    let config = Config::from_cli(&run_cmd)?;
    start_all_services(config, rpc_client, account, run_cmd).await
}

/// Prints information about the bot parameters.
fn print_app_title(account_address: Felt, network: NetworkName) {
    println!("\n
██╗   ██╗███████╗███████╗██╗   ██╗    ██╗     ██╗ ██████╗ ██╗   ██╗██╗██████╗  █████╗ ████████╗ ██████╗ ██████╗
██║   ██║██╔════╝██╔════╝██║   ██║    ██║     ██║██╔═══██╗██║   ██║██║██╔══██╗██╔══██╗╚══██╔══╝██╔═══██╗██╔══██╗
██║   ██║█████╗  ███████╗██║   ██║    ██║     ██║██║   ██║██║   ██║██║██║  ██║███████║   ██║   ██║   ██║██████╔╝
╚██╗ ██╔╝██╔══╝  ╚════██║██║   ██║    ██║     ██║██║▄▄ ██║██║   ██║██║██║  ██║██╔══██║   ██║   ██║   ██║██╔══██╗
 ╚████╔╝ ███████╗███████║╚██████╔╝    ███████╗██║╚██████╔╝╚██████╔╝██║██████╔╝██║  ██║   ██║   ╚██████╔╝██║  ██║
  ╚═══╝  ╚══════╝╚══════╝ ╚═════╝     ╚══════╝╚═╝ ╚══▀▀═╝  ╚═════╝ ╚═╝╚═════╝ ╚═╝  ╚═╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝

  🤖 Liquidator 👉 0x{:x}
  🎯 On {}", account_address, network);
}
