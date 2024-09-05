use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;

use crate::config::Config;

#[derive(Default, Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub address: Felt,
    pub amount: BigDecimal,
    pub decimals: i64,
}

impl Asset {
    pub fn from_address(config: &Config, address: Felt) -> Option<Self> {
        let name = config.get_asset_ticker_for_address(&address);
        let decimals = config.get_decimal_for_address(&address);

        match (name, decimals) {
            (Some(name), Some(decimals)) => Some(Self::new(name, address, decimals)),
            _ => None,
        }
    }

    pub fn new(name: String, address: Felt, decimals: i64) -> Self {
        Self {
            name,
            address,
            amount: BigDecimal::from(0),
            decimals,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use bigdecimal::BigDecimal;
    use starknet::core::types::Felt;

    use crate::{cli::NetworkName, config::Config};

    use super::Asset;

    #[test]
    fn test_asset_from_address() {
        let config = Config::new(
            NetworkName::Mainnet,
            crate::config::LiquidationMode::Full,
            &PathBuf::from("./config.yaml"),
        )
        .unwrap();
        let asset = Asset::from_address(
            &config,
            Felt::from_hex("0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(asset.name, "ETH");
        assert_eq!(
            asset.address,
            Felt::from_hex("0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7")
                .unwrap()
        );
        assert_eq!(asset.amount, BigDecimal::from(0));
        assert_eq!(asset.decimals, 18);
    }
}
