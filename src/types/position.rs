use anyhow::{anyhow, Ok, Result};
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
use std::ops::Neg;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::bindings::liquidate::{Liquidate, LiquidateParams, RouteNode, Swap, TokenAmount, I129};

use crate::config::{Config, LiquidationMode, LIQUIDATION_CONFIG_SELECTOR};
use crate::services::oracle::LatestOraclePrices;
use crate::storages::Storage;
use crate::utils::apply_overhead;
use crate::utils::constants::VESU_RESPONSE_DECIMALS;
use crate::{types::asset::Asset, utils::conversions::apibara_field_as_felt};

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
        if liquidation_mode.as_bool() {
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
        let ekubo_response: EkuboApiGetRouteResponse = response.json().await?;
        Ok(ekubo_response.route)
    }

    /// Returns the TX necessary to liquidate this position (approve + liquidate).
    // See: https://github.com/vesuxyz/vesu-v1/blob/a2a59936988fcb51bc85f0eeaba9b87cf3777c49/src/singleton.cairo#L1624
    #[allow(unused)]
    pub async fn get_liquidation_txs(
        &self,
        account: &StarknetAccount,
        liquidate_contract: Felt,
        amount_to_liquidate: BigDecimal,
        minimum_collateral_to_retrieve: BigDecimal,
        profit_estimated: BigDecimal,
    ) -> Result<Vec<Call>> {
        // TODO: remove those line when vesu contract allow partial liquidation
        // Setting those value to 0 because vesu Liquidate contract required amount = 0 for both swap
        let amount_to_liquidate = BigDecimal::from(0);
        let profit_estimated = BigDecimal::from(0);

        // The amount is in negative because contract use a inverted route to ensure that we get the exact amount of debt token
        let liquidate_token = TokenAmount {
            token: cainome::cairo_serde::ContractAddress(self.debt.address),
            amount: I129::cairo_deserialize(
                &[Felt::from(
                    amount_to_liquidate
                        .clone()
                        .with_scale(0)
                        .neg()
                        .into_bigint_and_exponent()
                        .0,
                )],
                0,
            )?,
        };

        let withdraw_token = TokenAmount {
            token: cainome::cairo_serde::ContractAddress(self.collateral.address),
            amount: I129::cairo_deserialize(
                &[Felt::from(
                    profit_estimated
                        .clone()
                        .with_scale(0)
                        .into_bigint_and_exponent()
                        .0,
                )],
                0,
            )?,
        };

        // As mentionned before the route is inverted for precision purpose
        let liquidate_route: Vec<RouteNode> = Position::get_ekubo_route(
            amount_to_liquidate
                .clone()
                .with_scale(0)
                .into_bigint_and_exponent()
                .0
                .to_str_radix(10),
            self.debt.name.clone(),
            self.collateral.name.clone(),
        )
        .await?;
        let liquidate_limit: u128 = u128::MAX;

        let withdraw_route: Vec<RouteNode> = Position::get_ekubo_route(
            profit_estimated
                .clone()
                .with_scale(0)
                .into_bigint_and_exponent()
                .0
                .to_str_radix(10),
            self.debt.name.clone(),
            String::from("usdc"),
        )
        .await?;
        let withdraw_limit: u128 = u128::MAX;

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

        let min_col_to_retrieve: [u8; 32] = minimum_collateral_to_retrieve
            .as_bigint_and_exponent()
            .0
            .to_bytes_be()
            .1
            .try_into()
            .expect("failed to parse min col to retrieve");

        let liquidate_params = LiquidateParams {
            pool_id: self.pool_id,
            collateral_asset: cainome::cairo_serde::ContractAddress(self.collateral.address),
            debt_asset: cainome::cairo_serde::ContractAddress(self.debt.address),
            user: cainome::cairo_serde::ContractAddress(self.user_address),
            recipient: cainome::cairo_serde::ContractAddress(account.account_address()),
            min_collateral_to_receive: cainome::cairo_serde::U256::from_bytes_be(
                &min_col_to_retrieve,
            ),
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

    // TODO: finish and fix this e2e test

    // const DEVNET_IMAGE: &str = "shardlabs/starknet-devnet-rs";
    // const DEVNET_IMAGE_TAG: &str = "0.1.2";
    // const DEVNET_PORT: u16 = 5050;

    // const APIBARA_IMAGE: &str = "quay.io/apibara/starknet";
    // const APIBARA_IMAGE_TAG: &str = "latest";
    // const APIBARA_PORT: u16 = 7171;

    // const FORK_BLOCK: u32 = 657064;

    // #[rstest::fixture]
    // async fn starknet_devnet_container() -> ContainerAsync<GenericImage> {
    //     GenericImage::new(DEVNET_IMAGE, DEVNET_IMAGE_TAG)
    //         .with_wait_for(WaitFor::message_on_stdout("Starknet Devnet listening"))
    //         .with_exposed_port(DEVNET_PORT.into())
    //         .with_mapped_port(DEVNET_PORT, DEVNET_PORT.into())
    //         .with_cmd(vec![
    //             "--fork-network=https://starknet-mainnet.public.blastapi.io/rpc/v0_7",
    //             "--block-generation-on=transaction",
    //             "--seed=1",
    //             "--chain-id=MAINNET",
    //             &format!("--fork-block={FORK_BLOCK}"),
    //         ])
    //         .with_container_name("starknet-devnet")
    //         .start()
    //         .await
    //         .expect("Failed to start devnet")
    // }

    // #[rstest::fixture]
    // async fn apibara_container() -> ContainerAsync<GenericImage> {
    //     let mount = Mount::tmpfs_mount("/.tmp");
    //     GenericImage::new(APIBARA_IMAGE, APIBARA_IMAGE_TAG)
    //         .with_wait_for(WaitFor::message_on_stdout("starting server"))
    //         .with_exposed_port(APIBARA_PORT.into())
    //         .with_mapped_port(APIBARA_PORT, APIBARA_PORT.into())
    //         .with_env_var("TMPDIR", "/.tmp")
    //         .with_cmd(vec![
    //             "start",
    //             "--devnet",
    //             "--rpc=http://host.docker.internal:5050",
    //             "--wait-for-rpc",
    //             &format!(
    //                 "--dangerously-override-ingestion-start-block={}",
    //                 FORK_BLOCK + 1
    //             ),
    //         ])
    //         .with_container_name("apibara-devnet")
    //         .with_mount(mount)
    //         .start()
    //         .await
    //         .expect("Failed to start devnet")
    // }

    // #[derive(Debug, Clone, Default)]
    // struct LiquidatorBot {
    //     env_vars: HashMap<String, String>,
    //     cmds: Vec<String>,
    // }

    // impl LiquidatorBot {
    //     fn with_env_vars(mut self, vars: HashMap<String, String>) -> Self {
    //         self.env_vars = vars;
    //         self
    //     }
    //     fn with_onchain_network(mut self, network: &str) -> Self {
    //         self.cmds.push(format!("--network={network}"));
    //         self
    //     }
    //     fn with_rpc_url(mut self, rpc_url: &str) -> Self {
    //         self.cmds.push(format!("--rpc-url={rpc_url}"));
    //         self
    //     }
    //     fn with_starting_block(mut self, starting_block: u32) -> Self {
    //         self.cmds.push(format!("--starting-block={starting_block}"));
    //         self
    //     }
    //     fn with_pragma_base_url(mut self, pragma_base_url: &str) -> Self {
    //         self.cmds
    //             .push(format!("--pragma-api-base-url={pragma_base_url}"));
    //         self
    //     }
    //     fn with_account(mut self, address: &str, private_key: &str) -> Self {
    //         self.cmds.push(format!("--account-address={address}"));
    //         self.cmds.push(format!("--private-key={private_key}"));
    //         self
    //     }
    // }

    // impl Image for LiquidatorBot {
    //     fn name(&self) -> &str {
    //         "liquidator-bot-e2e"
    //     }

    //     fn tag(&self) -> &str {
    //         "latest"
    //     }

    //     fn ready_conditions(&self) -> Vec<WaitFor> {
    //         vec![WaitFor::seconds(30)]
    //     }

    //     fn env_vars(
    //         &self,
    //     ) -> impl IntoIterator<
    //         Item = (
    //             impl Into<std::borrow::Cow<'_, str>>,
    //             impl Into<std::borrow::Cow<'_, str>>,
    //         ),
    //     > {
    //         &self.env_vars
    //     }

    //     fn cmd(&self) -> impl IntoIterator<Item = impl Into<std::borrow::Cow<'_, str>>> {
    //         &self.cmds
    //     }
    // }

    // #[rstest::fixture]
    // async fn liquidator_bot_container(
    //     #[future] apibara_container: ContainerAsync<GenericImage>,
    // ) -> (ContainerAsync<GenericImage>, ContainerAsync<LiquidatorBot>) {
    //     let apibara = apibara_container.await;
    //     // 1. Build the local image
    //     println!(
    //         "Building liquidator bot image..., {:#?}",
    //         liquidator_dockerfile_path()
    //     );
    //     ImageBuilder::default()
    //         .with_build_name("liquidator-bot-e2e")
    //         .with_dockerfile(&liquidator_dockerfile_path())
    //         .build()
    //         .await;

    //     // 2. setup env vars
    //     dotenvy::dotenv().unwrap();
    //     let mut env_vars = HashMap::new();
    //     env_vars.insert(
    //         "PRAGMA_API_KEY".to_string(),
    //         env::var("PRAGMA_API_KEY").unwrap(),
    //     );
    //     env_vars.insert(
    //         "APIBARA_API_KEY".to_string(),
    //         env::var("APIBARA_API_KEY").unwrap(),
    //     );

    //     // 3. Run the container
    //     (
    //         apibara,
    //         LiquidatorBot::default()
    //             .with_env_vars(env_vars)
    //             .with_onchain_network("devnet")
    //             .with_rpc_url("http://host.docker.internal:5050")
    //             .with_starting_block(FORK_BLOCK)
    //             .with_pragma_base_url("https://api.dev.pragma.build")
    //             .with_account(
    //                 "0x14923a0e03ec4f7484f600eab5ecf3e4eacba20ffd92d517b213193ea991502",
    //                 "0xe5852452e0757e16b127975024ade3eb",
    //             )
    //             .with_name("liquidator-bot-e2e")
    //             .start()
    //             .await
    //             .unwrap(),
    //     )
    // }

    // #[rstest]
    // #[tokio::test]
    // #[traced_test]
    // async fn test_liquidate_position(
    //     #[future] liquidator_bot_container: (
    //         ContainerAsync<GenericImage>,
    //         ContainerAsync<LiquidatorBot>,
    //     ),
    //     #[future] starknet_devnet_container: ContainerAsync<GenericImage>,
    // ) {
    //     let _devnet = starknet_devnet_container.await;
    //     let (_apibara, _bot) = liquidator_bot_container.await;

    //     let devnet_url = Url::parse("http://127.0.0.1:5050").unwrap();

    //     let provider = Arc::new(JsonRpcClient::new(HttpTransport::new(devnet_url.clone())));

    //     // We use devnet first account with seed 1
    //     let signer = LocalWallet::from(SigningKey::from_secret_scalar(
    //         Felt::from_hex("0xc10662b7b247c7cecf7e8a30726cff12").unwrap(),
    //     ));
    //     let address =
    //         Felt::from_hex("0x260a8311b4f1092db620b923e8d7d20e76dedcc615fb4b6fdf28315b81de201")
    //             .unwrap();
    //     let mut account = SingleOwnerAccount::new(
    //         provider.clone(),
    //         signer.clone(),
    //         address,
    //         chain_id::MAINNET,
    //         ExecutionEncoding::New,
    //     );

    //     // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
    //     // block. Optionally change the target block to pending with the following line:
    //     account.set_block_id(BlockId::Tag(BlockTag::Pending));

    //     // Deploy liquidate contract
    //     let liquidate_contract_artifact: SierraClass = serde_json::from_reader(
    //         std::fs::File::open("abis/vesu_liquidate_Liquidate.contract_class.json").unwrap(),
    //     )
    //     .unwrap();

    //     let liquidate_class_hash = liquidate_contract_artifact.class_hash().unwrap();

    //     let account = Arc::new(account);

    //     // Auto impersonate
    //     enable_auto_impersonate(devnet_url.clone()).await;

    //     // Declare liquidate contract
    //     let flattened_class = liquidate_contract_artifact.flatten().unwrap();
    //     let compiled_class: CompiledClass = serde_json::from_reader(
    //         std::fs::File::open("abis/vesu_liquidate_Liquidate.compiled_contract_class.json")
    //             .unwrap(),
    //     )
    //     .unwrap();

    //     let _ = account
    //         .declare_v2(
    //             Arc::new(flattened_class),
    //             compiled_class.class_hash().unwrap(),
    //         )
    //         .send()
    //         .await
    //         .unwrap();

    //     let liquidate_contract_factory =
    //         ContractFactory::new(liquidate_class_hash, account.clone());

    //     liquidate_contract_factory
    //         .deploy_v1(
    //             vec![
    //                 Felt::from_hex(
    //                     "0x00000005dd3D2F4429AF886cD1a3b08289DBcEa99A294197E9eB43b0e0325b4b",
    //                 )
    //                 .unwrap(),
    //                 Felt::from_hex(
    //                     "0x02545b2e5d519fc230e9cd781046d3a64e092114f07e44771e0d719d148725ef",
    //                 )
    //                 .unwrap(),
    //             ],
    //             Felt::from_dec_str("0").unwrap(),
    //             false,
    //         )
    //         .send()
    //         .await
    //         .expect("Unable to deploy liquidate contract");

    //     // Deploy mock oracle contract
    //     let mock_oracle_contract_artifact: SierraClass = serde_json::from_reader(
    //         std::fs::File::open("abis/vesu_MockPragmaOracle.contract_class.json").unwrap(),
    //     )
    //     .unwrap();

    //     let mock_oracle_class_hash = mock_oracle_contract_artifact.class_hash().unwrap();

    //     // Declare Mock oracle
    //     let flattened_class = mock_oracle_contract_artifact.flatten().unwrap();
    //     let compiled_class: CompiledClass = serde_json::from_reader(
    //         std::fs::File::open("abis/vesu_MockPragmaOracle.compiled_contract_class.json").unwrap(),
    //     )
    //     .unwrap();

    //     let declare_res = account
    //         .declare_v2(
    //             Arc::new(flattened_class),
    //             compiled_class.class_hash().unwrap(),
    //         )
    //         .send()
    //         .await
    //         .unwrap();

    //     let mock_oracle_contract_factory =
    //         ContractFactory::new(mock_oracle_class_hash, account.clone());
    //     mock_oracle_contract_factory
    //         .deploy_v1(vec![], Felt::from_dec_str("0").unwrap(), false)
    //         .send()
    //         .await
    //         .expect("Unable to deploy mock oracle contract");

    //     // Upgrade pragma contract to mock oracle
    //     let admin_address =
    //         Felt::from_hex("0x02356b628D108863BAf8644c945d97bAD70190AF5957031f4852d00D0F690a77")
    //             .unwrap();
    //     let admin_account = SingleOwnerAccount::new(
    //         provider.clone(),
    //         signer,
    //         admin_address,
    //         chain_id::MAINNET,
    //         ExecutionEncoding::New,
    //     );
    //     let res = admin_account
    //         .execute_v1(vec![Call {
    //             to: Felt::from_hex(
    //                 "0x2a85bd616f912537c50a49a4076db02c00b29b2cdc8a197ce92ed1837fa875b",
    //             )
    //             .unwrap(),
    //             selector: get_selector_from_name("upgrade").unwrap(),
    //             calldata: vec![declare_res.class_hash],
    //         }])
    //         .send()
    //         .await
    //         .expect("Failed to upgrade pragma contract");
    //     wait_for_tx(res.transaction_hash, account.provider().clone())
    //         .await
    //         .unwrap();

    //     // retrieved position should be
    //     // collateral => ETH : "0.319860064647672274",
    //     // collateral => USDC : "300.484447",
    //     // lltv => 0.68 (debt can't be > to 68% of collateral value)

    //     // Make a position liquidatable
    //     let new_eth_usd_price = 100000000000; // 1000 USD
    //     set_pragma_price(
    //         account.clone(),
    //         account.provider().clone(),
    //         new_eth_usd_price,
    //     )
    //     .await;

    //     // Assert that the bot has liquidated the position

    //     sleep(Duration::from_secs(1000)).await;

    //     //TODO : Check Key
    //     assert!(logs_contain("[🔭 Monitoring] Liquidatable position found "));
    //     //TODO : Check profit
    //     assert!(logs_contain(
    //         "[🔭 Monitoring] Trying to liquidiate position for"
    //     ));
    //     //TODO : Check key + tx hash
    //     assert!(logs_contain("[🔭 Monitoring] ✅ Liquidated position"));
    // }

    // async fn enable_auto_impersonate(devnet_url: Url) {
    //     let client = reqwest::Client::new();

    //     let payload = serde_json::json!({
    //         "jsonrpc": "2.0",
    //         "id": "1",
    //         "method": "devnet_autoImpersonate",
    //         "params": {}
    //     });

    //     let _response = client
    //         .post(devnet_url)
    //         .json(&payload)
    //         .send()
    //         .await
    //         .expect("Failed to auto impersonate");
    // }

    // async fn set_pragma_price<A>(
    //     account: A,
    //     provider: Arc<JsonRpcClient<HttpTransport>>,
    //     price: u128,
    // ) where
    //     A: ConnectedAccount + Sync,
    // {
    //     let res = account
    //         .execute_v1(vec![Call {
    //             to: Felt::from_hex(
    //                 "0x2a85bd616f912537c50a49a4076db02c00b29b2cdc8a197ce92ed1837fa875b",
    //             )
    //             .unwrap(),
    //             selector: get_selector_from_name("set_price").unwrap(),
    //             calldata: vec![
    //                 cairo_short_string_to_felt("eth/usd").unwrap(),
    //                 Felt::from(price),
    //             ],
    //         }])
    //         .send()
    //         .await
    //         .expect("Failed to set price");
    //     wait_for_tx(res.transaction_hash, provider).await.unwrap();
    // }
}
