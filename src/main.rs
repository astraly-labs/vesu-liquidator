use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};
use url::Url;

use anyhow::Result;
use bigdecimal::BigDecimal;
use starknet::{
    core::types::{BlockId, BlockTag, EmittedEvent, EventFilter, Felt, FunctionCall},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
};

use vesu_liquidator::{
    oracle::PragmaOracle,
    utils::{
        get_asset_name_for_address, get_decimal_for_address, EXTENSION_CONTRACT,
        MODIFY_POSITION_EVENT, PUBLIC_MAINNET_RPC, VESU_POSITION_UNSAFE_SELECTOR,
        VESU_SINGLETON_CONTRACT,
    },
};

// Constants for execution.
// Only for testing purposes.
const FROM_BLOCK: BlockId = BlockId::Number(668220);
const TO_BLOCK: BlockId = BlockId::Tag(BlockTag::Latest);
const NB_EVENTS_TO_RETRIEVE: u64 = 100;

#[derive(Default, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Asset {
    name: String,
    address: Felt,
    amount: BigDecimal,
    decimals: u32,
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
    user_address: Felt,
    pool_id: Felt,
    collateral: Asset,
    debt: Asset,
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
    async fn ltv_ratio(&self, pragma_oracle: &PragmaOracle) -> Result<BigDecimal> {
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
    user: Felt,
    pool_id: Felt,
    collateral_asset_address: Felt,
    debt_asset_address: Felt,
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

pub struct LiquidatorBackend {
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    pragma_oracle: Arc<PragmaOracle>,
}

impl LiquidatorBackend {
    pub fn new(rpc_url: Url, api_key: String) -> LiquidatorBackend {
        let rpc_client = Arc::new(JsonRpcClient::new(HttpTransport::new(rpc_url)));
        let pragma_oracle = Arc::new(PragmaOracle::new(api_key));

        LiquidatorBackend {
            rpc_client,
            pragma_oracle,
        }
    }

    /// Retrieve all the ModifyPosition events emitted from the Vesu Singleton Contract.
    async fn retrieve_events(&self) -> Result<Vec<EmittedEvent>> {
        let key_filter = vec![vec![MODIFY_POSITION_EVENT.to_owned()]];
        let filter = EventFilter {
            from_block: Some(FROM_BLOCK),
            to_block: Some(TO_BLOCK),
            address: Some(VESU_SINGLETON_CONTRACT.to_owned()),
            keys: Some(key_filter),
        };
        let events = self
            .rpc_client
            .get_events(filter, None, NB_EVENTS_TO_RETRIEVE)
            .await?;

        // It can happens that some events are emitted from Vesu extensions. We exclude them here.
        let filtered_events = events
            .events
            .into_iter()
            .filter(|event| {
                event
                    .keys
                    .get(4)
                    .map_or(false, |&key| key != EXTENSION_CONTRACT.to_owned())
            })
            .collect::<Vec<_>>();

        Ok(filtered_events)
    }

    /// From a list of ModifyPosition events, returns all the positions per user.
    async fn get_positions(
        &self,
        events: Vec<EmittedEvent>,
    ) -> Result<HashMap<Felt, HashSet<Position>>> {
        let mut position_per_users: HashMap<Felt, HashSet<Position>> = HashMap::new();
        for event in events {
            let mut position = Position::from_event(&event.keys);
            position = self.update_position(position, &event.keys).await;
            let ltv_ratio = position.ltv_ratio(&self.pragma_oracle).await?;
            println!("Position: {:?}", position);
            println!("LTV Ratio: {}\n", ltv_ratio);
            position_per_users
                .entry(position.user_address)
                .or_default()
                .insert(position);
        }

        Ok(position_per_users)
    }

    /// Update a position given the latest data available.
    async fn update_position(&self, position: Position, event_keys: &[Felt]) -> Position {
        let get_position_request = &FunctionCall {
            contract_address: VESU_SINGLETON_CONTRACT.to_owned(),
            entry_point_selector: VESU_POSITION_UNSAFE_SELECTOR.to_owned(),
            calldata: GetPositionRequest::from_event_keys(event_keys).as_calldata(),
        };
        let result = self
            .rpc_client
            .call(get_position_request, BlockId::Tag(BlockTag::Latest))
            .await
            .expect("failed to request position state");

        // TODO: update position amounts from [result]
        println!("{:?}", result);

        // position.scale_decimals();

        position
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = Url::parse(PUBLIC_MAINNET_RPC).expect("failed to parse RPC URL");
    let api_key = env::var("PRAGMA_API_KEY")
        .expect("API key not found please set PRAGMA_API_KEY env variable");

    let liquidator = LiquidatorBackend::new(rpc_url, api_key);

    let events = liquidator.retrieve_events().await?;
    let _position_per_user = liquidator.get_positions(events).await?;

    Ok(())
}
