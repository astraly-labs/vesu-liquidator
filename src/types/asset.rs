use bigdecimal::BigDecimal;
use starknet::core::types::Felt;

use crate::config::Config;

#[derive(Default, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Asset {
    pub name: String,
    pub address: Felt,
    pub amount: BigDecimal,
    pub decimals: i64,
}

impl Asset {
    pub fn from_address(config: &Config, address: Felt) -> Option<Self> {
        let name = config.get_asset_name_for_address(&address);
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
