use anyhow::{anyhow, Result};
use clap::Args;
use starknet::core::types::Felt;
use std::{path::PathBuf, str::FromStr};

fn parse_str_to_felt(s: &str) -> Result<Felt> {
    Felt::from_str(s).map_err(|_| anyhow!("Could not convert {s} to Felt"))
}

#[derive(Clone, Debug, Args)]
pub struct AccountParams {
    /// Account address of the liquidator account
    #[clap(long, value_parser = parse_str_to_felt, value_name = "LIQUIDATOR ACCOUNT ADDRESS", requires = "private_key")]
    pub account_address: Felt,

    /// Private key of the liquidator account
    #[clap(long, value_parser = parse_str_to_felt, value_name = "LIQUIDATOR PRIVATE KEY", requires = "account_address")]
    pub private_key: Option<Felt>,

    /// Keystore path for the liquidator account
    #[clap(
        long,
        value_name = "LIQUIDATOR KEYSTORE",
        requires = "keystore_password"
    )]
    pub keystore_path: Option<PathBuf>,

    /// Keystore password for the liquidator account
    #[clap(
        long,
        value_name = "LIQUIDATOR KEYSTORE PASSWORD",
        requires = "keystore_path"
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
                anyhow!("Missing liquidator account key. Use either (--private-key) or (--keystore-path + --keystore-password).")
            ),
        }
    }
}