pub mod account;

use std::env;

use anyhow::{anyhow, Result};
use url::Url;

use account::AccountParams;

fn parse_url(s: &str) -> Result<Url, url::ParseError> {
    s.parse()
}

#[derive(Clone, Debug, clap::Parser)]
pub struct RunCmd {
    #[allow(missing_docs)]
    #[clap(flatten)]
    pub account_params: AccountParams,

    /// The network chain configuration.
    #[clap(long, short, value_name = "NETWORK NAME")]
    pub network: NetworkName,

    /// The rpc endpoint url.
    #[clap(long, value_parser = parse_url, value_name = "RPC URL")]
    pub rpc_url: Url,

    /// The block you want to start syncing from.
    #[clap(long, short, value_name = "BLOCK NUMBER")]
    pub starting_block: u64,

    /// Pragma API Key for indexing.
    #[clap(long, value_parser = parse_url, value_name = "PRAGMA API BASE URL")]
    pub pragma_api_base_url: Url,

    /// Apibara API Key for indexing.
    #[clap(long, value_name = "APIBARA API KEY")]
    pub apibara_api_key: Option<String>,

    /// Pragma API Key for indexing.
    #[clap(long, value_name = "PRAGMA API KEY")]
    pub pragma_api_key: Option<String>,
}

impl RunCmd {
    /// Validate CLI arguments
    pub fn validate(&mut self) -> Result<()> {
        self.account_params.validate()?;
        if self.pragma_api_key.is_none() {
            self.pragma_api_key = env::var("PRAGMA_API_KEY").ok();
        }
        if self.apibara_api_key.is_none() {
            self.apibara_api_key = env::var("APIBARA_API_KEY").ok();
        }
        if self.pragma_api_key.is_none() && self.apibara_api_key.is_none() {
            return Err(anyhow!("Both Pragma API Key and Apibara API Key are missing. Please provide at least one either via command line arguments or environment variables."));
        }
        Ok(())
    }
}

/// Starknet network name.
#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq)]
pub enum NetworkName {
    #[value(alias("mainnet"))]
    Mainnet,
    #[value(alias("sepolia"))]
    Sepolia,
}
