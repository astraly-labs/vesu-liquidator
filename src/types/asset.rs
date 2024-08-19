use anyhow::{anyhow, Result};
use bigdecimal::BigDecimal;
use starknet::core::types::Felt;

use crate::config::{get_asset_name_for_address, get_decimal_for_address};

#[derive(Default, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Asset {
    pub name: String,
    pub address: Felt,
    pub amount: BigDecimal,
    pub decimals: i64,
}

impl TryFrom<Felt> for Asset {
    type Error = anyhow::Error;

    fn try_from(value: Felt) -> Result<Self> {
        let name = get_asset_name_for_address(value)
            .ok_or_else(|| anyhow!("Failed to get asset name for address"))?;
        let decimals = get_decimal_for_address(value)
            .ok_or_else(|| anyhow!("Failed to get decimals for address"))?;

        Ok(Asset {
            name,
            address: value,
            amount: BigDecimal::from(0),
            decimals,
        })
    }
}
