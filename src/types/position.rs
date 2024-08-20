use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use anyhow::Result;
use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::BigDecimal;
use colored::Colorize;
use starknet::{accounts::Call, core::types::Felt};
use tokio::sync::RwLock;

use crate::{
    config::{FLASH_LOAN_SELECTOR, LIQUIDATE_SELECTOR, VESU_SINGLETON_CONTRACT},
    oracle::PragmaOracle,
    types::asset::Asset,
    utils::conversions::apibara_field_element_as_felt,
};

// Thread-safe wrapper around the positions.
// PositionsMap is a map between position key <=> position.
pub struct PositionsMap(pub Arc<RwLock<HashMap<u64, Position>>>);

impl PositionsMap {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    pub async fn insert(&self, position: Position) -> Option<Position> {
        self.0.write().await.insert(position.key(), position)
    }
}

impl Default for PositionsMap {
    fn default() -> Self {
        Self::new()
    }
}

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

    /// Computes the liquidable amount for the liquidable position.
    pub async fn liquidable_amount(&self, pragma_oracle: &PragmaOracle) -> Result<BigDecimal> {
        let max_debt_in_dollar = self.collateral.amount.clone()
            * self.lltv.clone()
            * pragma_oracle
                .get_dollar_price(self.collateral.name.to_lowercase())
                .await?;
        let current_debt = self.debt.amount.clone()
            * pragma_oracle
                .get_dollar_price(self.debt.name.to_lowercase())
                .await?;
        Ok(current_debt - (max_debt_in_dollar + 1)) // +1 to be slighly under threshold
    }

    /// Check if a position is closed.
    pub fn is_closed(&self) -> bool {
        (self.collateral.amount == BigDecimal::from(0)) && (self.debt.amount == BigDecimal::from(0))
    }

    /// Returns if the position is liquidable or not.
    pub async fn is_liquidable(&self, pragma_oracle: &PragmaOracle) -> bool {
        let ltv_ratio = self
            .ltv(pragma_oracle)
            .await
            .expect("failed to retrieve ltv ratio");

        let is_liquidable = ltv_ratio > self.lltv;
        self.debug_position_state(is_liquidable, ltv_ratio);
        is_liquidable
    }

    /// Prints the status of the position and if it's liquidable or not.
    fn debug_position_state(&self, is_liquidable: bool, ltv_ratio: BigDecimal) {
        println!(
            "{} is currently at ratio {:.2}%/{:.2}% => {}",
            self,
            ltv_ratio * BigDecimal::from(100),
            self.lltv.clone() * BigDecimal::from(100),
            if is_liquidable {
                "is liquidable".green()
            } else {
                "is NOT liquidable".red()
            }
        );
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

    /// Returns the TX necessary to liquidate this position (flashloan + liquidate).
    pub fn get_liquidation_txs(
        &self,
        liquidator_address: Felt,
        amount_to_liquidate: BigDecimal,
    ) -> Vec<Call> {
        // https://docs.vesu.xyz/dev-guides/singleton#flash_loan
        let flash_loan_call = Call {
            to: VESU_SINGLETON_CONTRACT.to_owned(),
            selector: FLASH_LOAN_SELECTOR.to_owned(),
            calldata: vec![
                liquidator_address,                       // receiver
                self.debt.address,                        // asset
                Felt::from(amount_to_liquidate.digits()), // amount
                Felt::ZERO,                               // is_legacy
                Felt::ZERO,                               // data (?)
            ],
        };

        // https://docs.vesu.xyz/dev-guides/singleton#liquidate_position
        // https://github.com/vesuxyz/vesu-v1/blob/a2a59936988fcb51bc85f0eeaba9b87cf3777c49/src/data_model.cairo#L127C26-L127C27
        // TODO: Parameters need to be adjusted
        let liquidate_call = Call {
            to: VESU_SINGLETON_CONTRACT.to_owned(),
            selector: LIQUIDATE_SELECTOR.to_owned(),
            calldata: vec![
                self.pool_id,            // pool_id
                self.collateral.address, // collateral_asset
                self.debt.address,       // debt_asset
                self.user_address,       // user
                Felt::ZERO,              // receive_as_shares
                Felt::ZERO,              // data
            ],
        };
        vec![flash_loan_call, liquidate_call]
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Position {}/{} of user {:?}",
            self.collateral.name, self.debt.name, self.user_address
        )
    }
}
