use anyhow::{anyhow, Ok, Result};
use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::{BigDecimal, FromPrimitive};
use colored::Colorize;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use starknet::core::types::{BlockId, BlockTag, FunctionCall};
use starknet::core::types::{Call, Felt};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::bindings::liquidate::{Liquidate, LiquidateParams, RouteNode, Swap, TokenAmount};

use crate::config::{
    Config, LIQUIDATION_CONFIG_SELECTOR, VESU_LTV_CONFIG_SELECTOR, VESU_POSITION_UNSAFE_SELECTOR,
};
use crate::services::oracle::LatestOraclePrices;
use crate::storages::Storage;
use crate::utils::constants::{I129_ZERO, U256_ZERO, VESU_RESPONSE_DECIMALS};
use crate::utils::ekubo::get_ekubo_route;
use crate::{types::asset::Asset, utils::conversions::apibara_field_as_felt};

use super::account::StarknetAccount;

/// Threshold for which we consider a position almost liquidable.
const ALMOST_LIQUIDABLE_THRESHOLD: f64 = 0.035;

/// Thread-safe wrapper around the positions.
/// PositionsMap is a map between position position_key <=> position.
pub struct PositionsMap(pub Arc<DashMap<u64, Position>>);

impl PositionsMap {
    pub fn new() -> Self {
        Self(Arc::new(DashMap::new()))
    }

    pub fn from_storage(storage: &dyn Storage) -> Self {
        let positions = storage.get_positions();
        let dash_map = DashMap::new();
        for (key, value) in positions {
            dash_map.insert(key, value);
        }
        Self(Arc::new(dash_map))
    }

    pub fn insert(&self, position: Position) -> Option<Position> {
        self.0.insert(position.key(), position)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for PositionsMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
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

        let collateral_price = oracle_prices
            .0
            .get(&collateral_name)
            .ok_or_else(|| anyhow!("Price not found for collateral: {}", collateral_name))?
            .clone();

        let debt_price = oracle_prices
            .0
            .get(&debt_name)
            .ok_or_else(|| anyhow!("Price not found for debt: {}", debt_name))?
            .clone();

        let ltv = (&self.debt.amount * debt_price) / (&self.collateral.amount * collateral_price);
        Ok(ltv)
    }

    /// Check if a position is closed.
    pub fn is_closed(&self) -> bool {
        (self.collateral.amount == 0.into()) && (self.debt.amount == 0.into())
    }

    /// Returns if the position is liquidable or not.
    pub async fn is_liquidable(&self, oracle_prices: &LatestOraclePrices) -> bool {
        let ltv_ratio = self
            .ltv(oracle_prices)
            .await
            .expect("failed to retrieve ltv ratio");

        let is_liquidable = ltv_ratio >= self.lltv.clone();
        let is_almost_liquidable = ltv_ratio
            >= self.lltv.clone() - BigDecimal::from_f64(ALMOST_LIQUIDABLE_THRESHOLD).unwrap();
        if is_liquidable || is_almost_liquidable {
            self.logs_liquidation_state(is_liquidable, is_almost_liquidable, ltv_ratio);
        }
        is_liquidable
    }

    /// Logs the status of the position and if it's liquidable or not.
    fn logs_liquidation_state(
        &self,
        is_liquidable: bool,
        is_almost_liquidable: bool,
        ltv_ratio: BigDecimal,
    ) {
        tracing::info!(
            "{} is at ratio {:.2}%/{:.2}% => {}",
            self,
            ltv_ratio * BigDecimal::from(100),
            self.lltv.clone() * BigDecimal::from(100),
            if is_liquidable {
                "liquidable!".green()
            } else if is_almost_liquidable {
                "almost liquidable 🔫".yellow()
            } else {
                "NOT liquidable.".red()
            }
        );
    }

    // TODO : put that in cache in a map with poolid/collateral/debt as key
    /// Fetches the liquidation factor from the extension contract
    pub async fn fetch_liquidation_factors(
        &self,
        config: &Config,
        rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    ) -> BigDecimal {
        let calldata = vec![self.pool_id, self.collateral.address, self.debt.address];

        let liquidation_config_request = &FunctionCall {
            contract_address: config.extension_address,
            entry_point_selector: *LIQUIDATION_CONFIG_SELECTOR,
            calldata,
        };

        let ltv_config = rpc_client
            .call(liquidation_config_request, BlockId::Tag(BlockTag::Pending))
            .await
            .expect("failed to retrieve");
        BigDecimal::new(ltv_config[0].to_bigint(), VESU_RESPONSE_DECIMALS)
    }

    pub async fn update(
        &mut self,
        rpc_client: &Arc<JsonRpcClient<HttpTransport>>,
        singleton_address: &Felt,
    ) -> anyhow::Result<()> {
        self.update_amounts(rpc_client, singleton_address).await?;
        self.update_lltv(rpc_client, singleton_address).await?;
        Ok(())
    }

    async fn update_amounts(
        &mut self,
        rpc_client: &Arc<JsonRpcClient<HttpTransport>>,
        singleton_address: &Felt,
    ) -> anyhow::Result<()> {
        let get_position_request = &FunctionCall {
            contract_address: *singleton_address,
            entry_point_selector: *VESU_POSITION_UNSAFE_SELECTOR,
            calldata: self.as_update_calldata(),
        };
        let result = rpc_client
            .call(get_position_request, BlockId::Tag(BlockTag::Pending))
            .await?;

        let new_collateral = BigDecimal::new(result[4].to_bigint(), self.collateral.decimals);
        let new_debt = BigDecimal::new(result[6].to_bigint(), self.debt.decimals);
        self.collateral.amount = new_collateral;
        self.debt.amount = new_debt;
        Ok(())
    }

    async fn update_lltv(
        &mut self,
        rpc_client: &Arc<JsonRpcClient<HttpTransport>>,
        singleton_address: &Felt,
    ) -> anyhow::Result<()> {
        let ltv_config_request = &FunctionCall {
            contract_address: *singleton_address,
            entry_point_selector: *VESU_LTV_CONFIG_SELECTOR,
            calldata: self.as_ltv_calldata(),
        };

        let ltv_config = rpc_client
            .call(ltv_config_request, BlockId::Tag(BlockTag::Pending))
            .await?;

        self.lltv = BigDecimal::new(ltv_config[0].to_bigint(), VESU_RESPONSE_DECIMALS);
        Ok(())
    }

    /// Returns a unique identifier for the position by hashing the update calldata.
    pub fn key(&self) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.as_update_calldata().hash(&mut hasher);
        hasher.finish()
    }

    /// Returns the TX necessary to liquidate this position using the Vesu Liquidate
    /// contract.
    pub async fn get_vesu_liquidate_tx(
        &self,
        account: &StarknetAccount,
        liquidate_contract: Felt,
    ) -> Result<Call> {
        let route: Vec<RouteNode> = get_ekubo_route(
            self.debt.amount.clone(),
            self.debt.decimals,
            self.debt.name.clone(),
            self.collateral.name.clone(),
        )
        .await?;

        let liquidate_swap = Swap {
            route,
            token_amount: TokenAmount {
                token: cainome::cairo_serde::ContractAddress(self.debt.address),
                amount: I129_ZERO,
            },
            limit_amount: u128::MAX,
        };

        let liquidate_params = LiquidateParams {
            pool_id: self.pool_id,
            collateral_asset: cainome::cairo_serde::ContractAddress(self.collateral.address),
            debt_asset: cainome::cairo_serde::ContractAddress(self.debt.address),
            user: cainome::cairo_serde::ContractAddress(self.user_address),
            recipient: cainome::cairo_serde::ContractAddress(account.account_address()),
            min_collateral_to_receive: U256_ZERO,
            debt_to_repay: U256_ZERO,
            liquidate_swap,
            withdraw_swap: Swap {
                route: vec![],
                token_amount: TokenAmount {
                    token: cainome::cairo_serde::ContractAddress(self.debt.address),
                    amount: I129_ZERO,
                },
                limit_amount: u128::MAX,
            },
        };

        let liquidate_contract = Liquidate::new(liquidate_contract, account.0.clone());
        let liquidate_call = liquidate_contract.liquidate_getcall(&liquidate_params);

        Ok(liquidate_call)
    }

    /// Returns the position as a calldata for the LTV config RPC call.
    fn as_ltv_calldata(&self) -> Vec<Felt> {
        vec![self.pool_id, self.collateral.address, self.debt.address]
    }

    /// Returns the position as a calldata for the Update Position RPC call.
    fn as_update_calldata(&self) -> Vec<Felt> {
        vec![
            self.pool_id,
            self.collateral.address,
            self.debt.address,
            self.user_address,
        ]
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
