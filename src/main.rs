pub mod cli;
pub mod config;
pub mod display;
pub mod oracle;
pub mod services;
pub mod types;
pub mod utils;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use cli::RunCmd;
use services::start_liquidator_services;
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
use types::account::StarknetAccount;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    let mut run_cmd: RunCmd = RunCmd::parse();
    run_cmd.validate()?;

    display::print_app_title();

    let rpc_client = Arc::new(JsonRpcClient::new(HttpTransport::new(
        run_cmd.rpc_url.clone(),
    )));
    let account = StarknetAccount::from_cli(Arc::clone(&rpc_client), run_cmd.clone())?;

    start_liquidator_services(rpc_client, account, run_cmd).await
}
