pub mod account;

use bigdecimal::BigDecimal;
use std::path::PathBuf;
use url::Url;

use anyhow::{anyhow, Result};
use strum::Display;

use account::AccountParams;

use crate::config::LiquidationMode;
use crate::services::oracle::OracleMode;

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
    #[clap(long, short, value_name = "NETWORK NAME", env = "NETWORK")]
    pub network: NetworkName,

    /// The rpc endpoint url.
    #[clap(long, value_parser = parse_url, value_name = "RPC URL", env = "RPC_URL")]
    pub rpc_url: Url,

    /// Configuration file path.
    #[clap(
        long,
        default_value = "config.yaml",
        value_name = "VESU CONFIG PATH",
        env = "CONFIG_PATH"
    )]
    pub config_path: Option<PathBuf>,

    /// Configuration file path.
    #[clap(
        long,
        default_value = "data/db.json",
        value_name = "STORAGE PATH",
        env = "STORAGE_PATH"
    )]
    pub storage_path: Option<PathBuf>,

    /// The block you want to start syncing from.
    #[clap(
        long,
        short,
        value_name = "BLOCK NUMBER",
        default_value = "0",
        env = "STARTING_BLOCK"
    )]
    pub starting_block: u64,

    /// Pragma API base URL for indexing.
    #[clap(
        long,
        value_name = "PRAGMA API BASE URL",
        default_value = "dev.pragma.build",
        env = "PRAGMA_API_BASE"
    )]
    pub pragma_api_base: String,

    /// Apibara API Key for indexing.
    #[clap(long, value_name = "APIBARA API KEY", env = "APIBARA_API_KEY")]
    pub apibara_api_key: Option<String>,

    /// Pragma API Key for indexing.
    #[clap(long, value_name = "PRAGMA API KEY", env = "PRAGMA_API_KEY")]
    pub pragma_api_key: Option<String>,

    /// The interval in seconds for checking positions.
    #[clap(
        long,
        default_value = "10",
        value_name = "SECONDS",
        env = "CHECK_POSITIONS_INTERVAL"
    )]
    pub check_positions_interval: u64,

    /// The minimum profit required to execute a liquidation (in USD).
    #[clap(long, default_value = "0", value_name = "USD", env = "MIN_PROFIT")]
    pub min_profit: BigDecimal,

    /// Configuration file path.
    #[clap(long, value_enum, default_value_t = LiquidationMode::Full, value_name = "LIQUIDATION MODE", env = "LIQUIDATION_MODE")]
    pub liquidation_mode: LiquidationMode,

    /// The mode for the oracle service (http or websocket).
    #[clap(long, value_enum, default_value_t = OracleMode::Http, value_name = "ORACLE MODE", env = "ORACLE_MODE")]
    pub oracle_mode: OracleMode,

    /// The interval in seconds for updating oracle prices in http mode.
    #[clap(
        long,
        default_value = "30",
        value_name = "SECONDS",
        env = "PRICES_UPDATE_INTERVAL"
    )]
    pub prices_update_interval: u64,
}

/// First blocks with Vesu activity. Not necessary to index before.
const FIRST_MAINNET_BLOCK: u64 = 654244;
const FIRST_SEPOLIA_BLOCK: u64 = 77860;

impl RunCmd {
    pub fn validate(&mut self) -> Result<()> {
        self.account_params.validate()?;

        // Check and prompt for API keys
        if self.pragma_api_key.is_none() {
            return Err(anyhow!("Pragma API Key is missing. Please provide it via command line argument or set the PRAGMA_API_KEY environment variable."));
        }

        if self.apibara_api_key.is_none() {
            return Err(anyhow!("Apibara API Key is missing. Please provide it via command line argument or set the APIBARA_API_KEY environment variable."));
        }

        // Check RPC URL
        if self.rpc_url.to_string().is_empty() {
            return Err(anyhow!(
                "RPC URL is missing. Please provide it using the --rpc-url argument or set the RPC_URL environment variable."
            ));
        }

        // Check config path
        if self.config_path.is_none() {
            return Err(anyhow!("Config path is missing. Please provide it using the --config-path argument, set the CONFIG_PATH environment variable, or ensure the default 'config.yaml' file exists."));
        }

        // Check storage path
        if self.storage_path.is_none() {
            return Err(anyhow!("Storage path is missing. Please provide it using the --storage-path argument, set the STORAGE_PATH environment variable, or ensure the default 'data/db.json' file exists."));
        }

        // Adjust starting block based on network
        match self.network {
            NetworkName::Mainnet => {
                if self.starting_block <= FIRST_MAINNET_BLOCK {
                    self.starting_block = FIRST_MAINNET_BLOCK;
                }
            }
            NetworkName::Sepolia => {
                if self.starting_block <= FIRST_SEPOLIA_BLOCK {
                    self.starting_block = FIRST_SEPOLIA_BLOCK;
                }
            }
            #[cfg(feature = "testing")]
            NetworkName::Devnet => {
                if self.starting_block <= FIRST_MAINNET_BLOCK {
                    self.starting_block = FIRST_MAINNET_BLOCK;
                }
            }
        }

        println!("⚙️ Configuration after validation:");
        println!("{:#?}", self);

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
    #[cfg(feature = "testing")]
    #[value(alias("devnet"))]
    Devnet,
}
