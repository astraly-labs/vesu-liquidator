use std::collections::HashMap;
use std::fs;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;

pub const PUBLIC_MAINNET_RPC: &str = "https://starknet-mainnet.public.blastapi.io";

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub vesu: VesuConfig,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VesuConfig {
    pub position_unsafe_selector: String,
    pub modify_poisition_event: String,
    pub mainnet: NetworkConfig,
    pub sepolia: NetworkConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkConfig {
    pub singleton_address: String,
    pub extension_address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Asset {
    pub name: String,
    pub ticker: String,
    pub decimals: u32,
    pub mainnet_address: String,
    pub sepolia_address: String,
}

lazy_static! {
    pub static ref CONFIG: Config = {
        let config_str = fs::read_to_string("config.yaml").expect("Failed to read config file");
        serde_yaml::from_str(&config_str).expect("Failed to parse config")
    };
    pub static ref EXTENSION_CONTRACT: Felt =
        Felt::from_hex(&CONFIG.vesu.mainnet.extension_address).unwrap();
    pub static ref MODIFY_POSITION_EVENT: Felt =
        Felt::from_hex(&CONFIG.vesu.modify_poisition_event).unwrap();
    pub static ref VESU_SINGLETON_CONTRACT: Felt =
        Felt::from_hex(&CONFIG.vesu.mainnet.singleton_address).unwrap();
    pub static ref VESU_POSITION_UNSAFE_SELECTOR: Felt =
        Felt::from_hex(&CONFIG.vesu.position_unsafe_selector).unwrap();
    pub static ref ASSET_MAP: HashMap<Felt, &'static Asset> = {
        let mut map = HashMap::new();
        for asset in &CONFIG.assets {
            let address = Felt::from_hex(&asset.mainnet_address).unwrap();
            map.insert(address, asset);
        }
        map
    };
}

pub fn get_asset_name_for_address(address: Felt) -> Option<String> {
    ASSET_MAP.get(&address).map(|asset| asset.ticker.clone())
}

pub fn get_decimal_for_address(address: Felt) -> Option<u32> {
    ASSET_MAP.get(&address).map(|asset| asset.decimals)
}
