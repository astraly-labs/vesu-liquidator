use anyhow::{anyhow, Result};
use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::BigDecimal;
use colored::Colorize;
use starknet::accounts::Call;
use starknet::core::types::Felt;
use starknet::core::utils::get_selector_from_name;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::services::oracle::LatestOraclePrices;
use crate::utils::apply_overhead;
use crate::utils::conversions::big_decimal_to_u256;
use crate::{
    config::LIQUIDATE_SELECTOR, types::asset::Asset, utils::conversions::apibara_field_as_felt,
};

/// Thread-safe wrapper around the positions.
/// PositionsMap is a map between position position_key <=> position.
pub struct PositionsMap(pub Arc<RwLock<HashMap<u64, Position>>>);

impl PositionsMap {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    pub async fn insert(&self, position: Position) -> Option<Position> {
        self.0.write().await.insert(position.key(), position)
    }

    pub async fn len(&self) -> usize {
        self.0.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.0.read().await.is_empty()
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
    pub fn from_event(config: &Config, event_keys: &[FieldElement]) -> Option<Position> {
        let event_keys: Vec<Felt> = event_keys.iter().map(apibara_field_as_felt).collect();

        let collateral = Asset::from_address(config, event_keys[2]);
        let debt = Asset::from_address(config, event_keys[3]);
        if collateral.is_none() || debt.is_none() {
            return None;
        }

        let position = Position {
            pool_id: event_keys[1],
            collateral: collateral.unwrap(),
            debt: debt.unwrap(),
            user_address: event_keys[4],
            lltv: BigDecimal::default(),
        };
        Some(position)
    }

    /// Computes & returns the LTV Ratio for a position.
    pub async fn ltv(&self, oracle_prices: &LatestOraclePrices) -> Result<BigDecimal> {
        let collateral_name = self.collateral.name.to_lowercase();
        let debt_name = self.debt.name.to_lowercase();

        let prices = oracle_prices.0.lock().await;
        let collateral_as_dollars = prices
            .get(&collateral_name)
            .ok_or_else(|| anyhow!("Price not found for collateral: {}", collateral_name))?
            .clone();
        let debt_as_dollars = prices
            .get(&debt_name)
            .ok_or_else(|| anyhow!("Price not found for debt: {}", debt_name))?
            .clone();
        drop(prices);

        Ok((self.debt.amount.clone() * debt_as_dollars)
            / (self.collateral.amount.clone() * collateral_as_dollars))
    }

    /// Computes the liquidable amount for the liquidable position.
    pub async fn liquidable_amount(
        &self,
        oracle_prices: &LatestOraclePrices,
    ) -> Result<BigDecimal> {
        let prices = oracle_prices.0.lock().await;
        let collateral_dollar_price = prices
            .get(&self.collateral.name.to_lowercase())
            .ok_or_else(|| anyhow!("Price not found for collateral: {}", self.collateral.name))?
            .clone();
        let debt_asset_dollar_price = prices
            .get(&self.debt.name.to_lowercase())
            .ok_or_else(|| anyhow!("Price not found for debt: {}", self.debt.name))?
            .clone();
        drop(prices);

        let max_debt_in_dollar = &self.collateral.amount * &self.lltv * collateral_dollar_price;

        let current_debt = &self.debt.amount * debt_asset_dollar_price.clone();
        let liquidable_debt_in_dollar = current_debt - max_debt_in_dollar;

        let liquidable_amount =
            (&liquidable_debt_in_dollar / debt_asset_dollar_price).round(self.debt.decimals);

        Ok(apply_overhead(liquidable_amount))
    }

    /// Check if a position is closed.
    pub fn is_closed(&self) -> bool {
        (self.collateral.amount == BigDecimal::from(0)) && (self.debt.amount == BigDecimal::from(0))
    }

    /// Returns if the position is liquidable or not.
    pub async fn is_liquidable(&self, oracle_prices: &LatestOraclePrices) -> bool {
        let ltv_ratio = self
            .ltv(oracle_prices)
            .await
            .expect("failed to retrieve ltv ratio");

        let is_liquidable = ltv_ratio > self.lltv;
        if is_liquidable {
            self.debug_position_state(is_liquidable, ltv_ratio);
        }
        is_liquidable
    }

    /// Prints the status of the position and if it's liquidable or not.
    fn debug_position_state(&self, is_liquidable: bool, ltv_ratio: BigDecimal) {
        tracing::info!(
            "{} is at ratio {:.2}%/{:.2}% => {}",
            self,
            ltv_ratio * BigDecimal::from(100),
            self.lltv.clone() * BigDecimal::from(100),
            if is_liquidable {
                "liquidable!".green()
            } else {
                "NOT liquidable.".red()
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

    /// Returns the TX necessary to liquidate this position (approve + liquidate).
    // TODO: Flash loan with a custom contract with a on_flash_loan function.
    // See: https://github.com/vesuxyz/vesu-v1/blob/a2a59936988fcb51bc85f0eeaba9b87cf3777c49/src/singleton.cairo#L1624
    pub fn get_liquidation_txs(
        &self,
        singleton_contract: Felt,
        amount_to_liquidate: BigDecimal,
    ) -> Vec<Call> {
        let debt_to_repay = big_decimal_to_u256(amount_to_liquidate);

        let approve_call = Call {
            to: self.debt.address,
            selector: get_selector_from_name("approve").unwrap(),
            calldata: vec![
                singleton_contract,
                Felt::from(debt_to_repay.low()),
                Felt::from(debt_to_repay.high()),
            ],
        };

        // https://docs.vesu.xyz/dev-guides/singleton#liquidate_position
        let liquidate_call = Call {
            to: singleton_contract,
            selector: *LIQUIDATE_SELECTOR,
            calldata: vec![
                self.pool_id,            // pool_id
                self.collateral.address, // collateral_asset
                self.debt.address,       // debt_asset
                self.user_address,       // user
                Felt::ZERO,              // receive_as_shares
                Felt::from(4),           // number of elements below (two U256, low/high)
                Felt::ZERO,              // min_collateral (U256)
                Felt::ZERO,
                Felt::from(debt_to_repay.low()), // debt (U256)
                Felt::from(debt_to_repay.high()),
            ],
        };

        vec![approve_call, liquidate_call]
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
