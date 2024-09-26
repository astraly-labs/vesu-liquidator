use anyhow::{anyhow, Context, Ok, Result};
use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::num_bigint::BigInt;
use bigdecimal::{BigDecimal, FromPrimitive};
use cainome::cairo_serde::{ContractAddress, U256};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use starknet::core::types::{BlockId, BlockTag, FunctionCall};
use starknet::core::types::{Call, Felt};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::bindings::liquidate::{
    Liquidate, LiquidateParams, PoolKey, RouteNode, Swap, TokenAmount, I129,
};

use crate::config::{Config, LiquidationMode, LIQUIDATION_CONFIG_SELECTOR};
use crate::services::oracle::LatestOraclePrices;
use crate::storages::Storage;
use crate::utils::apply_overhead;
use crate::utils::constants::VESU_RESPONSE_DECIMALS;
use crate::utils::conversions::big_decimal_to_felt;
use crate::{types::asset::Asset, utils::conversions::apibara_field_as_felt};

use super::account::StarknetAccount;

/// Thread-safe wrapper around the positions.
/// PositionsMap is a map between position position_key <=> position.
pub struct PositionsMap(pub Arc<RwLock<HashMap<u64, Position>>>);

impl PositionsMap {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    pub fn from_storage(storage: &dyn Storage) -> Self {
        let positions = storage.get_positions();
        Self(Arc::new(RwLock::new(positions)))
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

        let prices = oracle_prices.0.lock().await;
        let collateral_price = prices
            .get(&collateral_name)
            .ok_or_else(|| anyhow!("Price not found for collateral: {}", collateral_name))?
            .clone();
        let debt_price = prices
            .get(&debt_name)
            .ok_or_else(|| anyhow!("Price not found for debt: {}", debt_name))?
            .clone();
        drop(prices);

        let ltv = (&self.debt.amount * debt_price) / (&self.collateral.amount * collateral_price);
        Ok(ltv)
    }

    /// Computes the liquidable amount for the liquidable position.
    /// (not accounting for price impact/slippage from swapping)
    pub async fn liquidable_amount(
        &self,
        liquidation_mode: LiquidationMode,
        oracle_prices: &LatestOraclePrices,
    ) -> Result<(BigDecimal, BigDecimal)> {
        let prices = oracle_prices.0.lock().await;
        let collateral_dollar_price = prices
            .get(&self.collateral.name.to_lowercase())
            .ok_or_else(|| anyhow!("Price not found for collateral: {}", self.collateral.name))?
            .clone();
        let debt_dollar_price = prices
            .get(&self.debt.name.to_lowercase())
            .ok_or_else(|| anyhow!("Price not found for debt: {}", self.debt.name))?
            .clone();
        drop(prices);

        let collateral_factor = self.lltv.clone();
        let total_collateral_value_in_usd =
            self.collateral.amount.clone() * collateral_dollar_price.clone();
        if liquidation_mode == LiquidationMode::Full {
            let total_collateral_value_in_usd = apply_overhead(total_collateral_value_in_usd);
            return Ok((
                total_collateral_value_in_usd.clone() / debt_dollar_price,
                total_collateral_value_in_usd.clone() / collateral_dollar_price,
            ));
        }
        let current_debt_in_usd = self.debt.amount.clone() * debt_dollar_price.clone();
        let maximum_health_factor = BigDecimal::new(BigInt::from(1001), 3);

        let liquidation_amount_in_usd = ((collateral_factor.clone()
            * total_collateral_value_in_usd)
            - (maximum_health_factor.clone() * current_debt_in_usd))
            / (collateral_factor - maximum_health_factor);

        let liquidation_amount_in_usd = apply_overhead(liquidation_amount_in_usd);
        let liquidatable_amount_in_debt_asset =
            (liquidation_amount_in_usd.clone() / debt_dollar_price).round(self.debt.decimals);
        let liquidatable_amount_in_collateral_asset =
            (liquidation_amount_in_usd / collateral_dollar_price).round(self.collateral.decimals);
        Ok((
            liquidatable_amount_in_debt_asset,
            liquidatable_amount_in_collateral_asset,
        ))
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

        let is_liquidable = ltv_ratio.clone() + BigDecimal::from_f64(0.1).unwrap() >= self.lltv;
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

    // TODO : put that in cache in a map with poolid/collateral/debt as key
    // Fetch liquidation factor from extension contract
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

    pub async fn get_ekubo_route(
        amount_as_string: String,
        from_token: String,
        to_token: String,
    ) -> Result<Vec<RouteNode>> {
        let ekubo_api_endpoint = format!(
            "https://mainnet-api.ekubo.org/quote/{amount_as_string}/{from_token}/{to_token}"
        );
        let http_client = reqwest::Client::new();

        let response = http_client.get(ekubo_api_endpoint).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("API request failed with status: {}", response.status());
        }

        let response_text = response.text().await?;

        let json_value: Value = serde_json::from_str(&response_text)?;

        // TODO: Horrible - refacto
        let route = json_value["route"]
            .as_array()
            .context("'route' is not an array")?
            .iter()
            .map(|node| {
                let pool_key = &node["pool_key"];
                Ok(RouteNode {
                    pool_key: PoolKey {
                        token0: ContractAddress(Felt::from_hex(
                            pool_key["token0"]
                                .as_str()
                                .context("token0 is not a string")?,
                        )?),
                        token1: ContractAddress(Felt::from_hex(
                            pool_key["token1"]
                                .as_str()
                                .context("token1 is not a string")?,
                        )?),
                        fee: u128::from_str_radix(
                            pool_key["fee"]
                                .as_str()
                                .context("fee is not a string")?
                                .trim_start_matches("0x"),
                            16,
                        )
                        .context("Failed to parse fee as u128")?,
                        tick_spacing: pool_key["tick_spacing"]
                            .as_u64()
                            .context("tick_spacing is not a u64")?
                            as u128,
                        extension: ContractAddress(Felt::from_hex(
                            pool_key["extension"]
                                .as_str()
                                .context("extension is not a string")?,
                        )?),
                    },
                    sqrt_ratio_limit: U256::from_bytes_be(
                        &Felt::from_hex(
                            node["sqrt_ratio_limit"]
                                .as_str()
                                .context("sqrt_ratio_limit is not a string")?,
                        )
                        .unwrap()
                        .to_bytes_be(),
                    ),
                    skip_ahead: node["skip_ahead"]
                        .as_u64()
                        .context("skip_ahead is not a u64")?
                        as u128,
                })
            })
            .collect::<Result<Vec<RouteNode>>>()?;

        Ok(route)
    }

    /// Returns the TX necessary to liquidate this position (approve + liquidate).
    // See: https://github.com/vesuxyz/vesu-v1/blob/a2a59936988fcb51bc85f0eeaba9b87cf3777c49/src/singleton.cairo#L1624
    pub async fn get_liquidation_txs(
        &self,
        account: &StarknetAccount,
        liquidate_contract: Felt,
        amount_to_liquidate: BigDecimal,
        minimum_collateral_to_retrieve: BigDecimal,
    ) -> Result<Vec<Call>> {
        // The amount is in negative because contract use a inverted route to ensure that we get the exact amount of debt token
        let liquidate_token = TokenAmount {
            token: cainome::cairo_serde::ContractAddress(self.debt.address),
            amount: I129 { mag: 0, sign: true },
        };

        let withdraw_token = TokenAmount {
            token: cainome::cairo_serde::ContractAddress(self.collateral.address),
            amount: I129 { mag: 0, sign: true },
        };

        // As mentionned before the route is inverted for precision purpose
        let liquidate_route: Vec<RouteNode> = Position::get_ekubo_route(
            String::from("10"), // TODO: ?
            self.debt.name.clone(),
            self.collateral.name.clone(),
        )
        .await?;

        let withdraw_route: Vec<RouteNode> = Position::get_ekubo_route(
            String::from("10"), // TODO: ?
            self.debt.name.clone(),
            String::from("usdc"),
        )
        .await?;

        let liquidate_contract = Liquidate::new(liquidate_contract, account.0.clone());

        let liquidate_swap = Swap {
            route: liquidate_route,
            token_amount: liquidate_token,
            limit_amount: u128::MAX,
        };
        let withdraw_swap = Swap {
            route: withdraw_route,
            token_amount: withdraw_token,
            limit_amount: u128::MAX,
        };

        let min_col_to_retrieve = big_decimal_to_felt(minimum_collateral_to_retrieve);

        let debt_to_repay = big_decimal_to_felt(amount_to_liquidate);

        let liquidate_params = LiquidateParams {
            pool_id: self.pool_id,
            collateral_asset: cainome::cairo_serde::ContractAddress(self.collateral.address),
            debt_asset: cainome::cairo_serde::ContractAddress(self.debt.address),
            user: cainome::cairo_serde::ContractAddress(self.user_address),
            recipient: cainome::cairo_serde::ContractAddress(account.account_address()),
            min_collateral_to_receive: cainome::cairo_serde::U256::from_bytes_be(
                &min_col_to_retrieve.to_bytes_be(),
            ),
            liquidate_swap,
            withdraw_swap,
            debt_to_repay: cainome::cairo_serde::U256::from_bytes_be(&debt_to_repay.to_bytes_be()),
        };

        let liquidate_call = liquidate_contract.liquidate_getcall(&liquidate_params);

        Ok(vec![liquidate_call])
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

#[cfg(test)]
mod tests {

    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use bigdecimal::{num_bigint::BigInt, BigDecimal};
    use starknet::core::types::Felt;
    use tokio::sync::Mutex;

    use crate::{
        cli::NetworkName,
        config::{Config, LiquidationMode},
        services::oracle::LatestOraclePrices,
        types::{asset::Asset, position::Position},
    };

    #[tokio::test]
    async fn test_liquidable() {
        let config = Config::new(
            NetworkName::Mainnet,
            LiquidationMode::Full,
            &PathBuf::from("./config.yaml"),
        )
        .unwrap();
        let mut eth = Asset::from_address(
            &config,
            Felt::from_hex("0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7")
                .unwrap(),
        )
        .unwrap();
        eth.amount = BigDecimal::new(BigInt::from(3), 1);
        let mut usdc = Asset::from_address(
            &config,
            Felt::from_hex("0x053c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8")
                .unwrap(),
        )
        .unwrap();
        usdc.amount = BigDecimal::new(BigInt::from(300), 0);
        let position = Position {
            user_address: Felt::from_hex(
                "0x14923a0e03ec4f7484f600eab5ecf3e4eacba20ffd92d517b213193ea991502",
            )
            .unwrap(),
            pool_id: Felt::from_hex(
                "0x4dc4f0ca6ea4961e4c8373265bfd5317678f4fe374d76f3fd7135f57763bf28",
            )
            .unwrap(),
            collateral: eth, //ETH
            debt: usdc,
            lltv: BigDecimal::new(BigInt::from(68), 2),
        };

        let mut oracle_price: HashMap<String, BigDecimal> = HashMap::new();
        oracle_price.insert("eth".to_string(), BigDecimal::new(BigInt::from(2000), 0));
        oracle_price.insert("usdc".to_string(), BigDecimal::new(BigInt::from(1), 0));

        let last_oracle_price = LatestOraclePrices(Arc::new(Mutex::new(oracle_price)));
        // Test Ltv computation
        assert_eq!(
            position.ltv(&last_oracle_price).await.unwrap(),
            BigDecimal::new(BigInt::from(5), 1)
        );
        // Test is not liquidatable
        assert!(!(position.is_liquidable(&last_oracle_price).await));
        // changing price to test a non liquidable position
        {
            last_oracle_price
                .0
                .lock()
                .await
                .insert("eth".to_string(), BigDecimal::new(BigInt::from(1000), 0));
        }
        //check new ltv
        assert_eq!(
            position.ltv(&last_oracle_price).await.unwrap(),
            BigDecimal::from(1)
        );
        //check that its liquidatable
        assert!(position.is_liquidable(&last_oracle_price).await);
        // changing price to test a non liquidable position

        let (amount_as_debt, amount_as_collateral) = position
            .liquidable_amount(LiquidationMode::Full, &last_oracle_price)
            .await
            .unwrap();
        // should be 300 $ with 2% overhead => 306
        assert_eq!(amount_as_debt, BigDecimal::from(306)); // 306 USDC with 1 USDC = 1$
        assert_eq!(amount_as_collateral, BigDecimal::new(BigInt::from(306), 3));
        // 0,306 with 1ETH = 1000
    }
}
