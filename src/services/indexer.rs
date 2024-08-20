use std::sync::Arc;

use anyhow::Result;
use bigdecimal::BigDecimal;
use futures_util::TryStreamExt;
use starknet::core::types::Felt;
use starknet::{
    core::types::{BlockId, BlockTag, FunctionCall},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
};
use tokio::sync::mpsc::Sender;

use apibara_core::{
    node::v1alpha2::DataFinality,
    starknet::v1alpha2::{Block, Filter, HeaderFilter},
};
use apibara_sdk::{configuration, ClientBuilder, Configuration, Uri};

use crate::{
    config::{
        MODIFY_POSITION_EVENT, VESU_LTV_CONFIG_SELECTOR, VESU_POSITION_UNSAFE_SELECTOR,
        VESU_SINGLETON_CONTRACT,
    },
    types::position::Position,
    utils::conversions::{apibara_field_element_as_felt, felt_as_apibara_field_element},
};

const INDEXING_STREAM_CHUNK_SIZE: usize = 128;

pub struct IndexerService {
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    uri: Uri,
    apibara_api_key: String,
    stream_config: Configuration<Filter>,
    positions_sender: Sender<Position>,
}

impl IndexerService {
    pub fn new(
        rpc_client: Arc<JsonRpcClient<HttpTransport>>,
        apibara_api_key: String,
        positions_sender: Sender<Position>,
        from_block: u64,
    ) -> IndexerService {
        // TODO: change if sepolia to https://sepolia.starknet.a5a.ch
        let uri: Uri = Uri::from_static("https://mainnet.starknet.a5a.ch");

        let stream_config = Configuration::<Filter>::default()
            .with_starting_block(from_block)
            .with_finality(DataFinality::DataStatusPending)
            // TODO: Filter does not seem to do anything. Done manually; investigate
            .with_filter(|mut filter| {
                filter
                    .with_header(HeaderFilter::weak())
                    .add_event(|event| {
                        event
                            .with_from_address(felt_as_apibara_field_element(
                                &VESU_SINGLETON_CONTRACT,
                            ))
                            .with_keys(vec![felt_as_apibara_field_element(&MODIFY_POSITION_EVENT)])
                    })
                    .build()
            });

        IndexerService {
            rpc_client,
            uri,
            apibara_api_key,
            stream_config,
            positions_sender,
        }
    }

    /// Retrieve all the ModifyPosition events emitted from the Vesu Singleton Contract.
    pub async fn start(self) -> Result<()> {
        let (config_client, config_stream) = configuration::channel(INDEXING_STREAM_CHUNK_SIZE);
        config_client
            .send(self.stream_config.clone())
            .await
            .unwrap();

        let mut stream = ClientBuilder::default()
            .with_bearer_token(Some(self.apibara_api_key.clone()))
            .connect(self.uri.clone())
            .await
            .unwrap()
            .start_stream::<Filter, Block, _>(config_stream)
            .await
            .unwrap();

        loop {
            match stream.try_next().await {
                Ok(Some(response)) => match response {
                    apibara_sdk::DataMessage::Data {
                        cursor: _,
                        end_cursor: _,
                        finality: _,
                        batch,
                    } => {
                        // TODO: Way better filtering :)
                        for block in batch {
                            for events_chunk in block.events {
                                for event in events_chunk.receipt.unwrap().events {
                                    // TODO: Currently hand filtered :)
                                    let from =
                                        apibara_field_element_as_felt(&event.from_address.unwrap());
                                    if from != VESU_SINGLETON_CONTRACT.to_owned() {
                                        continue;
                                    }
                                    let first = apibara_field_element_as_felt(&event.keys[0]);
                                    if first != MODIFY_POSITION_EVENT.to_owned() {
                                        continue;
                                    }
                                    let third = apibara_field_element_as_felt(&event.keys[3]);
                                    if third == Felt::ZERO {
                                        continue;
                                    }
                                    // Create the new position & update the fields.
                                    let mut new_position = Position::try_from_event(&event.keys)?;
                                    new_position = self.update_position(new_position).await?;
                                    let _ = self.positions_sender.try_send(new_position);
                                }
                            }
                        }
                    }
                    apibara_sdk::DataMessage::Invalidate { cursor } => match cursor {
                        Some(c) => {
                            return Err(anyhow::anyhow!(
                                "Received an invalidate request data at {}",
                                &c.order_key
                            ));
                        }
                        None => {
                            return Err(anyhow::anyhow!(
                                "Invalidate request without cursor provided"
                            ));
                        }
                    },
                    apibara_sdk::DataMessage::Heartbeat => {}
                },
                Ok(None) => continue,
                Err(e) => {
                    return Err(anyhow::anyhow!("Error while streaming: {}", e));
                }
            }
        }
    }

    /// Update a position given the latest data available.
    async fn update_position(&self, mut position: Position) -> Result<Position> {
        position = self.update_position_amounts(position).await?;
        position = self.update_position_lltv(position).await?;
        Ok(position)
    }

    /// Update the position debt & collateral amount with the latest available data.
    async fn update_position_amounts(&self, mut position: Position) -> Result<Position> {
        let get_position_request = &FunctionCall {
            contract_address: VESU_SINGLETON_CONTRACT.to_owned(),
            entry_point_selector: VESU_POSITION_UNSAFE_SELECTOR.to_owned(),
            calldata: position.as_update_calldata(),
        };
        let result = self
            .rpc_client
            .call(get_position_request, BlockId::Tag(BlockTag::Pending))
            .await?;

        let new_collateral = BigDecimal::new(result[4].to_bigint(), position.collateral.decimals);
        let new_debt = BigDecimal::new(result[6].to_bigint(), position.debt.decimals);
        position.collateral.amount = new_collateral;
        position.debt.amount = new_debt;
        Ok(position)
    }

    /// Update the LLTV with the latest available data.
    async fn update_position_lltv(&self, mut position: Position) -> Result<Position> {
        let ltv_config_request = &FunctionCall {
            contract_address: VESU_SINGLETON_CONTRACT.to_owned(),
            entry_point_selector: VESU_LTV_CONFIG_SELECTOR.to_owned(),
            calldata: position.as_ltv_calldata(),
        };

        let ltv_config = self
            .rpc_client
            .call(ltv_config_request, BlockId::Tag(BlockTag::Pending))
            .await?;

        // Decimals is always 18 for the ltv_config response
        position.lltv = BigDecimal::new(ltv_config[0].to_bigint(), 18);
        Ok(position)
    }
}
