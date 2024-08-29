use anyhow::{anyhow, Result};
use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::BigDecimal;
use colored::Colorize;
use starknet::accounts::{Account, Call, ConnectedAccount};
use starknet::core::types::{Felt, StarknetError};
use starknet::core::utils::get_selector_from_name;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::bindings::liquidate::{self, Liquidate, LiquidateParams, Swap};

use crate::config::Config;
use crate::services::oracle::LatestOraclePrices;
use crate::utils::apply_overhead;
use crate::utils::conversions::big_decimal_to_u256;
use crate::{
    config::LIQUIDATE_SELECTOR, types::asset::Asset, utils::conversions::apibara_field_as_felt,
};

use super::account::StarknetAccount;

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
        let collateral_price = prices
            .get(&self.collateral.name.to_lowercase())
            .ok_or_else(|| anyhow!("Price not found for collateral: {}", self.collateral.name))?
            .clone();
        let debt_price = prices
            .get(&self.debt.name.to_lowercase())
            .ok_or_else(|| anyhow!("Price not found for debt: {}", self.debt.name))?
            .clone();
        drop(prices);

        let current_debt = &self.debt.amount * debt_price.clone();
        let max_debt = &self.collateral.amount * &self.lltv * collateral_price;

        let liquidable_debt = current_debt - max_debt;
        let liquidable_amount = (&liquidable_debt / debt_price).round(self.debt.decimals);

        Ok(apply_overhead(liquidable_amount))
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
    // See: https://github.com/vesuxyz/vesu-v1/blob/a2a59936988fcb51bc85f0eeaba9b87cf3777c49/src/singleton.cairo#L1624
    pub fn get_liquidation_txs(
        &self,
        account: &StarknetAccount,
        singleton_contract: Felt,
        liquidate_contract: Felt,
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

        // let liquidate_contract = Liquidate::new(liquidate_contract, account);

        // let liquidate_swap = Swap{};
        // let withdraw_swap = Swap{};

        // let liquidate_params = LiquidateParams {
        //     pool_id : self.pool_id,
        //     collateral_asset: cainome::cairo_serde::ContractAddress(self.collateral.address),
        //     debt_asset: cainome::cairo_serde::ContractAddress(self.debt.address),
        //     user: cainome::cairo_serde::ContractAddress(self.user_address),
        //     recipient: cainome::cairo_serde::ContractAddress(account.account_address()),
        //     min_collateral_to_receive : cainome::cairo_serde::U256::try_from((Felt::ZERO,Felt::ZERO)).expect("failed to parse felt zero"),
        //     full_liquidation : false,
        //     liquidate_swap,
        //     withdraw_swap,
        // };

        // let liquidate_call = liquidate_contract.liquidate_getcall(&liquidate_params);

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

    use rstest::*;
    use testcontainers::core::wait::WaitFor;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::{ContainerAsync, GenericImage, ImageExt};

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

    #[rstest]
    #[tokio::test]
    async fn test_liquidate_position(
        #[future] starknet_devnet_container: ContainerAsync<GenericImage>,
    ) {
        let devnet = starknet_devnet_container.await;

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
                vec![Felt::ZERO, Felt::ZERO],
                Felt::from_dec_str("0").unwrap(),
                false,
            )
            .send()
            .await
            .expect("Unable to deploy contract");
    }
}
