use std::{path::PathBuf, str::FromStr};

use anyhow::{anyhow, Result};
use starknet::core::types::Felt;

fn parse_str_to_felt(s: &str) -> Result<Felt> {
    Felt::from_str(s).map_err(|_| anyhow!("Could not convert {s} to Felt"))
}

#[derive(Clone, Debug, clap::Args)]
pub struct AccountParams {
    /// Account address of the liquidator account
    #[clap(long, value_parser = parse_str_to_felt, value_name = "LIQUIDATOR ACCOUNT ADDRESS")]
    pub account_address: Option<Felt>,
    /// Account address of the liquidator account
    #[clap(long, value_parser = parse_str_to_felt, value_name = "LIQUIDATOR PRIVATE KEY")]
    pub private_key: Option<Felt>,
    /// Keystore path for the liquidator account
    #[clap(long, value_name = "LIQUIDATOR KEYSTORE")]
    pub keystore_path: Option<PathBuf>,
    /// Keystore password for the liquidator account
    #[clap(long, value_name = "LIQUIDATOR KEYSTORE PASSWORD")]
    pub keystore_password: Option<String>,
}
