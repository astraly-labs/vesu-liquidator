pub mod cli;
pub mod config;
pub mod services;
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

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    let mut run_cmd: RunCmd = RunCmd::parse();
    run_cmd.validate()?;

    print_app_title(
        run_cmd.account_params.account_address,
        run_cmd.network,
        run_cmd.starting_block,
    );

    let config = Config::from_cli(&run_cmd)?;

    let rpc_url = run_cmd.rpc_url.clone();
    let rpc_client = Arc::new(JsonRpcClient::new(HttpTransport::new(rpc_url)));
    let account = StarknetAccount::from_cli(rpc_client.clone(), run_cmd.clone())?;

    start_all_services(config, rpc_client, account, run_cmd).await
}
