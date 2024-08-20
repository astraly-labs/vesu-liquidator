pub mod cli;
pub mod config;
pub mod oracle;
pub mod services;
pub mod types;
pub mod utils;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use cli::{NetworkName, RunCmd};
use services::start_liquidator_services;
use starknet::{
    core::types::Felt,
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
};
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

    let rpc_client = Arc::new(JsonRpcClient::new(HttpTransport::new(
        run_cmd.rpc_url.clone(),
    )));
    let account = StarknetAccount::from_cli(Arc::clone(&rpc_client), run_cmd.clone())?;

    start_liquidator_services(rpc_client, account, run_cmd).await
}
