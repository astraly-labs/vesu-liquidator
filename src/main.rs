use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};
use url::Url;

use anyhow::Result;
use starknet::{
    core::types::{BlockId, BlockTag, EmittedEvent, EventFilter, Felt, FunctionCall},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
};

use vesu_liquidator::{
    oracle::PragmaOracle,
    types::{GetPositionRequest, Position},
    utils::{
        EXTENSION_CONTRACT, MODIFY_POSITION_EVENT, PUBLIC_MAINNET_RPC,
        VESU_POSITION_UNSAFE_SELECTOR, VESU_SINGLETON_CONTRACT,
    },
};

// Constants for execution.
// Only for testing purposes.
const FROM_BLOCK: BlockId = BlockId::Number(668220);
const TO_BLOCK: BlockId = BlockId::Tag(BlockTag::Latest);
const NB_EVENTS_TO_RETRIEVE: u64 = 100;

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
