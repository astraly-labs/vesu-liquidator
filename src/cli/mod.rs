pub mod account;

use std::{env, path::PathBuf};
use url::Url;

use anyhow::{anyhow, Result};
use strum::Display;

use account::AccountParams;

use crate::config::LiquidationMode;

fn parse_url(s: &str) -> Result<Url> {
    s.parse()
        .map_err(|_| anyhow!("Could not convert {s} to Url"))
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

    /// Configuration file path.
    #[clap(long, default_value = "config.yaml", value_name = "VESU CONFIG PATH")]
    pub config_path: Option<PathBuf>,

    /// Configuration file path.
    #[clap(long, default_value = "data.json", value_name = "STORAGE PATH")]
    pub storage_path: Option<PathBuf>,

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

    /// Configuration file path.
    #[clap(long, value_enum, default_value_t = LiquidationMode::FullLiquidation, value_name = "LIQUIDATION MODE")]
    pub liquidation_mode: LiquidationMode,
}

/// First blocks with Vesu activity. Not necessary to index before.
const FIRST_MAINNET_BLOCK: u64 = 654244;
const FIRST_SEPOLIA_BLOCK: u64 = 77860;

impl RunCmd {
    pub fn validate(&mut self) -> Result<()> {
        self.account_params.validate()?;
        if self.pragma_api_key.is_none() {
            self.pragma_api_key = env::var("PRAGMA_API_KEY").ok();
        }
        if self.apibara_api_key.is_none() {
            self.apibara_api_key = env::var("APIBARA_API_KEY").ok();
        }
        if self.pragma_api_key.is_none() || self.apibara_api_key.is_none() {
            return Err(anyhow!("Pragma API Key or Apibara API Key is missing. Please provide at least one via command line arguments or environment variable."));
        }

        match self.network {
            NetworkName::Mainnet => {
                if self.starting_block <= FIRST_MAINNET_BLOCK {
                    self.starting_block = FIRST_MAINNET_BLOCK;
                }
            }
            NetworkName::Devnet => {
                if self.starting_block <= FIRST_MAINNET_BLOCK {
                    self.starting_block = FIRST_MAINNET_BLOCK;
                }
            }
            NetworkName::Sepolia => {
                if self.starting_block <= FIRST_SEPOLIA_BLOCK {
                    self.starting_block = FIRST_SEPOLIA_BLOCK;
                }
            }
        }
        Ok(())
    }
}

/// Starknet network name.
#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Display)]
pub enum NetworkName {
    #[strum(serialize = "Mainnet")]
    #[value(alias("mainnet"))]
    Mainnet,
    #[strum(serialize = "Sepolia")]
    #[value(alias("sepolia"))]
    Sepolia,
    #[strum(serialize = "Devnet")]
    #[value(alias("devnet"))]
    Devnet,
}
