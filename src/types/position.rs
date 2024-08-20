use std::hash::{Hash, Hasher};

use anyhow::Result;
use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::BigDecimal;
use starknet::core::types::Felt;

use crate::{
    oracle::PragmaOracle, types::asset::Asset, utils::conversions::apibara_field_element_as_felt,
};

#[derive(Default, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Position {
    pub user_address: Felt,
    pub pool_id: Felt,
    pub collateral: Asset,
    pub debt: Asset,
    pub lltv: BigDecimal,
}

impl Position {
    /// Create a new position from the event_keys of a ModifyPosition event.
    pub fn try_from_event(event_keys: &[FieldElement]) -> Result<Position> {
        let event_keys: Vec<Felt> = event_keys
            .iter()
            .map(apibara_field_element_as_felt)
            .collect();
        let position = Position {
            pool_id: event_keys[1],
            collateral: Asset::try_from(event_keys[2])?,
            debt: Asset::try_from(event_keys[3])?,
            user_address: event_keys[4],
            lltv: BigDecimal::default(),
        };
        Ok(position)
    }

    /// Computes & returns the LTV Ratio for a position.
    pub async fn ltv(&self, pragma_oracle: &PragmaOracle) -> Result<BigDecimal> {
        let collateral_as_dollars = pragma_oracle
            .get_dollar_price(self.collateral.name.to_lowercase())
            .await?;

        let debt_as_dollars = pragma_oracle
            .get_dollar_price(self.debt.name.to_lowercase())
            .await?;

        Ok((self.debt.amount.clone() * debt_as_dollars)
            / (self.collateral.amount.clone() * collateral_as_dollars))
    }

    /// Check if a position is closed.
    pub fn is_closed(&self) -> bool {
        (self.collateral.amount == BigDecimal::from(0)) && (self.debt.amount == BigDecimal::from(0))
    }

    /// Returns the position as a calldata for the LTV config RPC call.
    pub fn as_ltv_calldata(&self) -> Vec<Felt> {
        vec![self.pool_id, self.collateral.address, self.debt.address]
    }

    /// Returns the position as a calldata for the Update Position RPC call.
    pub fn as_update_calldata(&self) -> Vec<Felt> {
        vec![
            self.pool_id,
            self.collateral.address,
            self.debt.address,
            self.user_address,
        ]
    }

    /// Returns a unique identifier for the position by hashing the update calldata.
    pub fn key(&self) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.as_update_calldata().hash(&mut hasher);
        hasher.finish()
    }
}
