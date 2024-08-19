use bigdecimal::BigDecimal;
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
    config::{
        EXTENSION_CONTRACT, MODIFY_POSITION_EVENT, PUBLIC_MAINNET_RPC,
        VESU_POSITION_UNSAFE_SELECTOR, VESU_SINGLETON_CONTRACT,
    },
    oracle::PragmaOracle,
    types::{GetPositionRequest, Position},
};

// Constants for execution.
// Only for testing purposes.
const FROM_BLOCK: BlockId = BlockId::Number(668220);
const TO_BLOCK: BlockId = BlockId::Tag(BlockTag::Latest);
const NB_EVENTS_TO_RETRIEVE: u64 = 100;

pub struct Liquidator {
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    pragma_oracle: Arc<PragmaOracle>,
}

impl Liquidator {
    pub fn new(rpc_url: Url, pragma_api_key: String) -> Liquidator {
        Liquidator {
            rpc_client: Arc::new(JsonRpcClient::new(HttpTransport::new(rpc_url))),
            pragma_oracle: Arc::new(PragmaOracle::new(pragma_api_key)),
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
            let mut position = Position::try_from_event(&event.keys)?;
            position = self.update_position(position, &event.keys).await;
            if position.is_closed() {
                continue;
            }
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
    async fn update_position(&self, mut position: Position, event_keys: &[Felt]) -> Position {
        let get_position_request = &FunctionCall {
            contract_address: VESU_SINGLETON_CONTRACT.to_owned(),
            entry_point_selector: VESU_POSITION_UNSAFE_SELECTOR.to_owned(),
            calldata: GetPositionRequest::try_from_event_keys(event_keys).as_calldata(),
        };
        let result = self
            .rpc_client
            .call(get_position_request, BlockId::Tag(BlockTag::Latest))
            .await
            .expect("failed to request position state");

        position.collateral.amount =
            BigDecimal::new(result[4].to_bigint(), position.collateral.decimals as i64);
        position.debt.amount =
            BigDecimal::new(result[6].to_bigint(), position.debt.decimals as i64);
        println!("{:?}", result);

        position
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url: Url = PUBLIC_MAINNET_RPC.parse()?;
    let pragmapi_key: String = env::var("PRAGMA_API_KEY")?;

    let liquidator = Liquidator::new(rpc_url, pragmapi_key);

    let events = liquidator.retrieve_events().await?;
    let _position_per_user = liquidator.get_positions(events).await?;

    Ok(())
}
