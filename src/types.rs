use anyhow::{anyhow, Result};
use bigdecimal::BigDecimal;
use starknet::core::types::Felt;

use crate::{
    config::{get_asset_name_for_address, get_decimal_for_address},
    oracle::PragmaOracle,
};

#[derive(Default, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Asset {
    pub name: String,
    pub address: Felt,
    pub amount: BigDecimal,
    pub decimals: u32,
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

#[derive(Default, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Position {
    pub user_address: Felt,
    pub pool_id: Felt,
    pub collateral: Asset,
    pub debt: Asset,
}

impl Position {
    pub fn try_from_event(event_keys: &[Felt]) -> Result<Position> {
        let position = Position {
            pool_id: event_keys[1],
            collateral: Asset::try_from(event_keys[2])?,
            debt: Asset::try_from(event_keys[3])?,
            user_address: event_keys[4],
        };
        Ok(position)
    }

    /// Computes & returns the LTV Ratio for a position.
    pub async fn ltv_ratio(&self, pragma_oracle: &PragmaOracle) -> Result<BigDecimal> {
        let collateral_as_dollars = pragma_oracle
            .get_dollar_price(self.collateral.name.to_lowercase())
            .await?;

        let debt_as_dollars = pragma_oracle
            .get_dollar_price(self.debt.name.to_lowercase())
            .await?;

        Ok((self.debt.amount.clone() * debt_as_dollars)
            / (self.collateral.amount.clone() * collateral_as_dollars))
    }

    pub fn is_closed(&self) -> bool {
        self.collateral.amount == BigDecimal::from(0) && self.debt.amount == BigDecimal::from(0)
    }
}

#[derive(Default, Clone, Hash, Eq, PartialEq, Debug)]
pub struct GetPositionRequest {
    pub user: Felt,
    pub pool_id: Felt,
    pub collateral_asset_address: Felt,
    pub debt_asset_address: Felt,
}

impl GetPositionRequest {
    pub fn try_from_event_keys(event_keys: &[Felt]) -> GetPositionRequest {
        GetPositionRequest {
            pool_id: event_keys[1],
            collateral_asset_address: event_keys[2],
            debt_asset_address: event_keys[3],
            user: event_keys[4],
        }
    }

    pub fn as_calldata(&self) -> Vec<Felt> {
        vec![
            self.pool_id,
            self.collateral_asset_address,
            self.debt_asset_address,
            self.user,
        ]
    }
}
