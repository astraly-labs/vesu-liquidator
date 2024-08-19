use anyhow::Result;
use bigdecimal::BigDecimal;
use starknet::core::types::Felt;

use crate::{
    oracle::PragmaOracle,
    utils::{get_asset_name_for_address, get_decimal_for_address},
};

#[derive(Default, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Asset {
    pub name: String,
    pub address: Felt,
    pub amount: BigDecimal,
    pub decimals: u32,
}

impl Asset {
    fn new(address: Felt) -> Asset {
        Asset {
            name: get_asset_name_for_address(address),
            address,
            amount: BigDecimal::from(0),
            decimals: get_decimal_for_address(address),
        }
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
    pub fn from_event(event_keys: &[Felt]) -> Position {
        let user_address = event_keys[4];
        let pool_id = event_keys[1];
        let collateral = Asset::new(event_keys[2]);
        let debt = Asset::new(event_keys[3]);

        Position {
            user_address,
            pool_id,
            collateral,
            debt,
        }
    }

    // /// Adapt the decimals between the collateral & debt asset.
    // /// For example, for ETH and USDT, if:
    // /// ETH: 12 decimals,
    // /// USDT: 6 decimals,
    // /// We want them to have the same decimals. So we add decimals to USDT and we'll have:
    // /// ETH: 12 decimals,
    // /// USDT: 12 decimals.
    // fn scale_decimals(&mut self) {
    //     if self.collateral.decimals > self.debt.decimals {
    //         self.debt.amount *= 10_u128.pow(self.collateral.decimals - self.debt.decimals);
    //     } else if self.collateral.decimals < self.debt.decimals {
    //         self.collateral.amount *= 10_u128.pow(self.debt.decimals - self.collateral.decimals);
    //     }
    // }

    /// Computes & returns the LTV Ratio for a position.
    pub async fn ltv_ratio(&self, pragma_oracle: &PragmaOracle) -> Result<BigDecimal> {
        let collateral_as_dollars = pragma_oracle
            .get_dollar_price(self.collateral.name.to_lowercase())
            .await?;

        let debt_as_dollars = pragma_oracle
            .get_dollar_price(self.debt.name.to_lowercase())
            .await?;

        println!(
            "{} * {}",
            self.collateral.amount.clone(),
            collateral_as_dollars
        );
        Ok((self.debt.amount.clone() * debt_as_dollars)
            / (self.collateral.amount.clone() * collateral_as_dollars))
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
    pub fn from_event_keys(event_keys: &[Felt]) -> GetPositionRequest {
        GetPositionRequest {
            user: event_keys[4],
            pool_id: event_keys[1],
            collateral_asset_address: event_keys[2],
            debt_asset_address: event_keys[3],
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
