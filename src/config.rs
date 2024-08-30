use std::fs;
use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;
use starknet::core::utils::get_selector_from_name;

use crate::cli::{NetworkName, RunCmd};

// Contract selectors
lazy_static! {
    pub static ref MODIFY_POSITION_EVENT: Felt = get_selector_from_name("ModifyPosition").unwrap();
    pub static ref VESU_POSITION_UNSAFE_SELECTOR: Felt =
        get_selector_from_name("position_unsafe").unwrap();
    pub static ref VESU_LTV_CONFIG_SELECTOR: Felt = get_selector_from_name("ltv_config").unwrap();
    pub static ref FLASH_LOAN_SELECTOR: Felt = get_selector_from_name("flash_loan").unwrap();
    pub static ref LIQUIDATE_SELECTOR: Felt = get_selector_from_name("liquidate_position").unwrap();
    pub static ref LIQUIDATION_CONFIG_SELECTOR: Felt =
        get_selector_from_name("liquidation_config").unwrap();
}

#[derive(Debug, Clone)]
pub struct Config {
    pub network: NetworkName,
    pub singleton_address: Felt,
    pub extension_address: Felt,
    pub liquidate_address: Felt,
    pub assets: Vec<Asset>,
    pub asset_map: HashMap<Felt, Asset>,
}

impl Config {
    pub fn from_cli(run_cmd: &RunCmd) -> Result<Self> {
        let config_path = run_cmd.config_path.clone().unwrap_or_default();
        let network = run_cmd.network;

        Self::new(network, &config_path)
    }

    pub fn new(network: NetworkName, config_path: &PathBuf) -> Result<Self> {
        let raw_config: RawConfig = {
            let config_str = fs::read_to_string(config_path)?;
            serde_yaml::from_str(&config_str)?
        };

        let network_config = match network {
            NetworkName::Mainnet => &raw_config.vesu.mainnet,
            NetworkName::Sepolia => &raw_config.vesu.sepolia,
        };

        let singleton_address = Felt::from_hex(&network_config.singleton_address)?;
        let extension_address = Felt::from_hex(&network_config.extension_address)?;
        let liquidate_address = Felt::from_hex(&network_config.liquidate_address)?;

        let assets = raw_config.assets;
        let asset_map = assets
            .iter()
            .filter_map(|asset| {
                let address = match network {
                    NetworkName::Mainnet => Felt::from_hex(&asset.mainnet_address),
                    NetworkName::Sepolia => Felt::from_hex(&asset.sepolia_address),
                };
                address.ok().map(|addr| (addr, asset.clone()))
            })
            .collect();

        let config = Config {
            network,
            singleton_address,
            extension_address,
            liquidate_address,
            assets,
            asset_map,
        };

        Ok(config)
    }

    pub fn get_asset_ticker_for_address(&self, address: &Felt) -> Option<String> {
        self.asset_map
            .get(address)
            .map(|asset| asset.ticker.clone())
    }

    pub fn get_decimal_for_address(&self, address: &Felt) -> Option<i64> {
        self.asset_map.get(address).map(|asset| asset.decimals)
    }
}

// Below are the structs that represents the raw config extracted from the yaml file.

#[derive(Debug, Deserialize, Serialize)]
pub struct RawConfig {
    pub vesu: VesuConfig,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VesuConfig {
    pub mainnet: NetworkConfig,
    pub sepolia: NetworkConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkConfig {
    pub singleton_address: String,
    pub extension_address: String,
    pub liquidate_address: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Asset {
    pub name: String,
    pub ticker: String,
    pub decimals: i64,
    pub mainnet_address: String,
    pub sepolia_address: String,
}
