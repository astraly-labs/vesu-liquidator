use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use apibara_core::starknet::v1alpha2::Event;
use apibara_core::{
    node::v1alpha2::DataFinality,
    starknet::v1alpha2::{Block, Filter, HeaderFilter},
};
use apibara_sdk::{configuration, ClientBuilder, Configuration, Uri};
use bigdecimal::BigDecimal;
use futures_util::TryStreamExt;
use starknet::core::types::Felt;
use starknet::{
    core::types::{BlockId, BlockTag, FunctionCall},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
};
use tokio::sync::mpsc::Sender;

use crate::cli::NetworkName;
use crate::config::{
    Config, MODIFY_POSITION_EVENT, VESU_LTV_CONFIG_SELECTOR, VESU_POSITION_UNSAFE_SELECTOR,
};
use crate::utils::constants::VESU_RESPONSE_DECIMALS;
use crate::{
    types::position::Position,
    utils::conversions::{apibara_field_as_felt, felt_as_apibara_field},
};

const INDEXING_STREAM_CHUNK_SIZE: usize = 1024;

pub struct IndexerService {
    config: Config,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    uri: Uri,
    apibara_api_key: String,
    stream_config: Configuration<Filter>,
    positions_sender: Sender<(u64, Position)>,
    seen_positions: HashSet<u64>,
}

impl IndexerService {
    pub fn new(
        config: Config,
        rpc_client: Arc<JsonRpcClient<HttpTransport>>,
        apibara_api_key: String,
        positions_sender: Sender<(u64, Position)>,
        from_block: u64,
    ) -> IndexerService {
        let uri = match config.network {
            NetworkName::Mainnet => Uri::from_static("https://mainnet.starknet.a5a.ch"),
            NetworkName::Sepolia => Uri::from_static("https://sepolia.starknet.a5a.ch"),
            NetworkName::Devnet => Uri::from_static("http://host.docker.internal:7171"),
        };

        let stream_config = Configuration::<Filter>::default()
            .with_starting_block(from_block)
            .with_finality(DataFinality::DataStatusPending)
            .with_filter(|mut filter| {
                filter
                    .with_header(HeaderFilter::weak())
                    .add_event(|event| {
                        event
                            .with_from_address(felt_as_apibara_field(&config.singleton_address))
                            .with_keys(vec![felt_as_apibara_field(&MODIFY_POSITION_EVENT)])
                    })
                    .build()
            });

        IndexerService {
            config,
            rpc_client,
            uri,
            apibara_api_key,
            stream_config,
            positions_sender,
            seen_positions: HashSet::default(),
        }
    }

    /// Retrieve all the ModifyPosition events emitted from the Vesu Singleton Contract.
    pub async fn start(mut self) -> Result<()> {
        let (config_client, config_stream) = configuration::channel(INDEXING_STREAM_CHUNK_SIZE);

        let mut reached_pending_block: bool = false;

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
                        finality,
                        batch,
                    } => {
                        if finality == DataFinality::DataStatusPending && !reached_pending_block {
                            self.log_pending_block_reached(batch.last());
                            reached_pending_block = true;
                        }
                        for block in batch {
                            for event in block.events {
                                if let Some(event) = event.event {
                                    let block_number = match block.header.clone() {
                                        Some(hdr) => hdr.block_number,
                                        None => 0,
                                    };
                                    self.create_position_from_event(block_number, event).await?;
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

    /// Index the provided event & creates a new position.
    async fn create_position_from_event(&mut self, block_number: u64, event: Event) -> Result<()> {
        if event.from_address.is_none() {
            return Ok(());
        }

        let debt_address = apibara_field_as_felt(&event.keys[3]);
        // Corresponds to event associated with the extension contract - we ignore them.
        if debt_address == Felt::ZERO {
            return Ok(());
        }

        // Create the new position & update the fields.
        if let Some(mut new_position) = Position::from_event(&self.config, &event.keys) {
            new_position = self.update_position(new_position).await?;
            if new_position.is_closed() {
                return Ok(());
            }
            let position_key = new_position.key();
            if self.seen_positions.insert(position_key) {
                tracing::info!("[🔍 Indexer] Found new position 0x{:x}", new_position.key());
            }
            match self.positions_sender.try_send((block_number, new_position)) {
                Ok(_) => {}
                Err(e) => panic!("Could not send position: {}", e),
            }
        }
        Ok(())
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
            contract_address: self.config.singleton_address,
            entry_point_selector: *VESU_POSITION_UNSAFE_SELECTOR,
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
            contract_address: self.config.singleton_address,
            entry_point_selector: *VESU_LTV_CONFIG_SELECTOR,
            calldata: position.as_ltv_calldata(),
        };

        let ltv_config = self
            .rpc_client
            .call(ltv_config_request, BlockId::Tag(BlockTag::Pending))
            .await?;

        position.lltv = BigDecimal::new(ltv_config[0].to_bigint(), VESU_RESPONSE_DECIMALS);
        Ok(position)
    }

    /// Logs that we successfully reached current pending block
    fn log_pending_block_reached(&self, last_block_in_batch: Option<&Block>) {
        let maybe_pending_block_number = if let Some(last_block) = last_block_in_batch {
            last_block.header.as_ref().map(|header| header.block_number)
        } else {
            None
        };

        if let Some(pending_block_number) = maybe_pending_block_number {
            tracing::info!(
                "[🔍 Indexer] 🥳🎉 Reached pending block #{}!",
                pending_block_number
            );
        } else {
            tracing::info!("[🔍 Indexer] 🥳🎉 Reached pending block!",);
        }
    }
}
