use std::{path::PathBuf, str::FromStr};

use anyhow::{anyhow, Result};
use clap::Args;
use starknet::core::types::Felt;

fn parse_felt(s: &str) -> Result<Felt> {
    Felt::from_str(s).map_err(|_| anyhow!("Could not convert {s} to Felt"))
}

#[derive(Clone, Debug, Args)]
pub struct AccountParams {
    /// Account address of the liquidator account
    #[clap(long, value_parser = parse_felt, value_name = "LIQUIDATOR ACCOUNT ADDRESS", env = "ACCOUNT_ADDRESS")]
    pub account_address: Felt,

    /// Private key of the liquidator account
    #[clap(long, value_parser = parse_felt, value_name = "LIQUIDATOR PRIVATE KEY", env = "PRIVATE_KEY")]
    pub private_key: Option<Felt>,

    /// Keystore path for the liquidator account
    #[clap(long, value_name = "LIQUIDATOR KEYSTORE", env = "KEYSTORE_PATH")]
    pub keystore_path: Option<PathBuf>,

    /// Keystore password for the liquidator account
    #[clap(
        long,
        value_name = "LIQUIDATOR KEYSTORE PASSWORD",
        env = "KEYSTORE_PASSWORD"
    )]
    pub keystore_password: Option<String>,
}

impl AccountParams {
    pub fn validate(&self) -> Result<()> {
        match (
            &self.private_key,
            &self.keystore_path,
            &self.keystore_password,
        ) {
            (Some(_), None, None) => Ok(()),
            (None, Some(_), Some(_)) => Ok(()),
            _ => Err(
                anyhow!("Missing liquidator account key. Use either (--private-key or PRIVATE_KEY env var) or (--keystore-path + --keystore-password or KEYSTORE_PATH + KEYSTORE_PASSWORD env vars).")
            ),
        }
    }
}
