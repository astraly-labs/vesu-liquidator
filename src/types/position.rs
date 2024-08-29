use anyhow::{anyhow, Result};
use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::num_bigint::BigInt;
use bigdecimal::BigDecimal;
use cainome::cairo_serde::CairoSerde;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use starknet::accounts::Call;
use starknet::core::types::Felt;
use starknet::core::types::{BlockId, BlockTag, FunctionCall};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Mul, Neg};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::bindings::liquidate::{Liquidate, LiquidateParams, RouteNode, Swap, TokenAmount, I129};

use crate::config::{Config, LIQUIDATION_CONFIG_SELECTOR};
use crate::services::oracle::LatestOraclePrices;
use crate::storages::Storage;
use crate::utils::apply_overhead;
use crate::utils::constants::VESU_RESPONSE_DECIMALS;
use crate::{
    types::asset::Asset, utils::conversions::apibara_field_as_felt,
};

use super::account::StarknetAccount;

/// Thread-safe wrapper around the positions.
/// PositionsMap is a map between position position_key <=> position.
pub struct PositionsMap(pub Arc<RwLock<HashMap<u64, Position>>>);

#[derive(Deserialize)]
pub struct EkuboApiGetRouteResponse {
    route: Vec<RouteNode>,
}

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
        oracle_prices: &LatestOraclePrices,
    ) -> Result<BigDecimal> {
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
        let current_debt_in_usd = self.debt.amount.clone() * debt_dollar_price.clone();
        let maximum_health_factor = BigDecimal::new(BigInt::from(999), 3);

        let liquidation_amount_in_usd = ((collateral_factor.clone()
            * total_collateral_value_in_usd)
            - (maximum_health_factor.clone() * current_debt_in_usd))
            / (collateral_factor - maximum_health_factor);

        let liquidation_amount_in_usd = apply_overhead(liquidation_amount_in_usd);
        Ok((liquidation_amount_in_usd / debt_dollar_price).round(self.debt.decimals))
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

    pub async fn get_ekubo_route(amount_as_string: String, from_token: String, to_token: String) -> Result<Vec<RouteNode>> {
        let ekubo_api_endpoint = format!("https://mainnet-api.ekubo.org/quote/{amount_as_string}/{from_token}/{to_token}");
        let http_client = reqwest::Client::new();
        let response = http_client.get(ekubo_api_endpoint).send().await?;
        let response_text = response.text().await?;
        let ekubo_response: EkuboApiGetRouteResponse = serde_json::from_str(&response_text)?;
        Ok(ekubo_response.route)
    }

    /// Returns the TX necessary to liquidate this position (approve + liquidate).
    // See: https://github.com/vesuxyz/vesu-v1/blob/a2a59936988fcb51bc85f0eeaba9b87cf3777c49/src/singleton.cairo#L1624
    pub async fn get_liquidation_txs(
        &self,
        account: &StarknetAccount,
        liquidate_contract: Felt,
        amount_to_liquidate: BigDecimal,
        collateral_retrieved: BigDecimal
    ) -> Result<Vec<Call>> {

        //putting the amount in negative because contract use a inverted route to ensure that we get the exact amount of debt token
        let liquidate_token = TokenAmount {
            token: cainome::cairo_serde::ContractAddress(self.debt.address),
            amount: I129::cairo_deserialize(
                &vec![Felt::from(amount_to_liquidate.clone().with_scale(0).neg().into_bigint_and_exponent().0)],
                0,
            )
            .expect("failed to deserialize amount to liquidiate"),
        };

        let withdraw_token = TokenAmount {
            token: cainome::cairo_serde::ContractAddress(self.collateral.address),
            amount: I129::cairo_deserialize(
                &vec![Felt::from(collateral_retrieved.clone().with_scale(0).into_bigint_and_exponent().0)],
                0,
            )
            .expect("failed to deserialize amount to liquidiate"),
        };

        //As mentionned before the route is inverted for precision purpose
        let liquidate_route : Vec<RouteNode> = Position::get_ekubo_route(amount_to_liquidate.clone().with_scale(0).into_bigint_and_exponent().0.to_str_radix(10), self.debt.name.clone(), self.collateral.name.clone()).await?;
        let liquidate_limit: u128 = u128::max_value();

        let withdraw_route : Vec<RouteNode> = Position::get_ekubo_route(collateral_retrieved.clone().with_scale(0).into_bigint_and_exponent().0.to_str_radix(10), self.debt.name.clone(), String::from("usdc")).await?;
        let withdraw_limit: u128 = u128::max_value();

        let liquidate_contract = Liquidate::new(liquidate_contract, account.0.clone());

        let liquidate_swap = Swap {
            route: liquidate_route,
            token_amount: liquidate_token,
            limit_amount: liquidate_limit,
        };
        let withdraw_swap = Swap {
            route: withdraw_route,
            token_amount: withdraw_token,
            limit_amount: withdraw_limit,
        };

        let liquidate_params = LiquidateParams {
            pool_id: self.pool_id,
            collateral_asset: cainome::cairo_serde::ContractAddress(self.collateral.address),
            debt_asset: cainome::cairo_serde::ContractAddress(self.debt.address),
            user: cainome::cairo_serde::ContractAddress(self.user_address),
            recipient: cainome::cairo_serde::ContractAddress(account.account_address()),
            min_collateral_to_receive: cainome::cairo_serde::U256::try_from((
                Felt::ZERO,
                Felt::ZERO,
            ))
            .expect("failed to parse felt zero"),
            full_liquidation: false,
            liquidate_swap,
            withdraw_swap,
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

    use starknet::{
        accounts::{ExecutionEncoding, SingleOwnerAccount},
        contract::ContractFactory,
        core::{
            chain_id,
            types::{contract::SierraClass, BlockId, BlockTag, Felt},
        },
        macros::felt,
        providers::{jsonrpc::HttpTransport, JsonRpcClient},
        signers::{LocalWallet, SigningKey},
    };
    use url::Url;
    use std::collections::HashMap;

    use rstest::*;
    use testcontainers::Image;
    use testcontainers::core::wait::WaitFor;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::{ContainerAsync, GenericImage, ImageExt};

    use crate::utils::test_utils::{ImageBuilder, liquidator_dockerfile_path};

    const DEVNET_IMAGE: &str = "shardlabs/starknet-devnet-rs";
    const DEVNET_IMAGE_TAG: &str = "latest";
    const DEVNET_PORT: u16 = 5050;

    #[rstest::fixture]
    async fn starknet_devnet_container() -> ContainerAsync<GenericImage> {
        GenericImage::new(DEVNET_IMAGE, DEVNET_IMAGE_TAG)
            .with_wait_for(WaitFor::message_on_stdout("Starknet Devnet listening"))
            .with_exposed_port(DEVNET_PORT.into())
            .with_mapped_port(DEVNET_PORT, DEVNET_PORT.into())
            .with_cmd(vec![
                "--fork-network=https://starknet-mainnet.public.blastapi.io/rpc/v0_7",
                "--seed=1",
            ])
            .start()
            .await
            .expect("Failed to start devnet")
    }

    #[derive(Debug, Clone)]
    struct LiquidatorBot {
        env_vars: HashMap<String, String>,
        cmds: Vec<String>,
    }

    // impl LiquidatorBot {
    //     fn with_account_address()
    // }

    // impl Image for LiquidatorBot {

    // }

    #[rstest::fixture]
    async fn liquidator_bot() -> ContainerAsync<GenericImage> {
        // 1. Build the local image
        ImageBuilder::default()
            .with_build_name("liquidator-bot-e2e")
            .with_dockerfile(&liquidator_dockerfile_path())
            .build()
            .await;

        // 2. Run the container
        LiquidatorBot::default()
            .with_container_name("liquidator-bot-container")
            .start()
            .await
            .unwrap()
    }

    #[rstest]
    #[tokio::test]
    async fn test_liquidate_position(
        #[future] starknet_devnet_container: ContainerAsync<GenericImage>,
    ) {
        let _devnet = starknet_devnet_container.await;

        let contract_artifact: SierraClass = serde_json::from_reader(
            std::fs::File::open("abis/vesu_liquidate_Liquidate.contract_class.json").unwrap(),
        )
        .unwrap();
        let class_hash = contract_artifact.class_hash().unwrap();

        let provider = JsonRpcClient::new(HttpTransport::new(
            Url::parse("http://127.0.0.1:5050").unwrap(),
        ));

        // We use devnet first account with seed 1
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex("0xc10662b7b247c7cecf7e8a30726cff12").unwrap(),
        ));
        let address =
            Felt::from_hex("0x260a8311b4f1092db620b923e8d7d20e76dedcc615fb4b6fdf28315b81de201")
                .unwrap();
        let mut account = SingleOwnerAccount::new(
            provider,
            signer,
            address,
            chain_id::MAINNET,
            ExecutionEncoding::New,
        );

        // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
        // block. Optionally change the target block to pending with the following line:
        account.set_block_id(BlockId::Tag(BlockTag::Pending));

        let contract_factory = ContractFactory::new(class_hash, account);
        contract_factory
            .deploy_v1(
                vec![
                    Felt::from_hex(
                        "0x00000005dd3D2F4429AF886cD1a3b08289DBcEa99A294197E9eB43b0e0325b4b",
                    )
                    .unwrap(),
                    Felt::ZERO,
                ],
                Felt::from_dec_str("0").unwrap(),
                false,
            )
            .send()
            .await
            .expect("Unable to deploy contract");

        // Make a position liquidable

        // Assert that the bot has liquidated the position
    }
}
