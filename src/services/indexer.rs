use anyhow::Result;
use apibara_core::starknet::v1alpha2::Event;
use apibara_core::{
    node::v1alpha2::DataFinality,
    starknet::v1alpha2::{Block, Filter, HeaderFilter},
};
use apibara_sdk::{configuration, ClientBuilder, Configuration, Uri};
use dashmap::DashSet;
use futures_util::TryStreamExt;
use starknet::core::types::Felt;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinSet;

use crate::cli::NetworkName;
use crate::config::{Config, MODIFY_POSITION_EVENT};
use crate::utils::services::Service;
use crate::{
    types::position::Position,
    utils::conversions::{apibara_field_as_felt, felt_as_apibara_field},
};

const INDEXING_STREAM_CHUNK_SIZE: usize = 1;

#[derive(Clone)]
pub struct IndexerService {
    config: Config,
    uri: Uri,
    apibara_api_key: String,
    stream_config: Configuration<Filter>,
    positions_sender: Sender<(u64, Position)>,
    seen_positions: DashSet<u64>,
}

#[async_trait::async_trait]
impl Service for IndexerService {
    async fn start(&mut self, join_set: &mut JoinSet<anyhow::Result<()>>) -> anyhow::Result<()> {
        let service = self.clone();
        join_set.spawn(async move {
            tracing::info!("🧩 Indexer service started");
            service.run_forever().await?;
            Ok(())
        });
        Ok(())
    }
}

impl IndexerService {
    pub fn new(
        config: Config,
        apibara_api_key: String,
        positions_sender: Sender<(u64, Position)>,
        from_block: u64,
    ) -> IndexerService {
        let uri = match config.network {
            NetworkName::Mainnet => Uri::from_static("https://mainnet.starknet.a5a.ch"),
            NetworkName::Sepolia => Uri::from_static("https://sepolia.starknet.a5a.ch"),
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
            uri,
            apibara_api_key,
            stream_config,
            positions_sender,
            seen_positions: DashSet::default(),
        }
    }

    /// Retrieve all the ModifyPosition events emitted from the Vesu Singleton Contract.
    pub async fn run_forever(mut self) -> Result<()> {
        let (config_client, config_stream) = configuration::channel(INDEXING_STREAM_CHUNK_SIZE);

        let mut reached_pending_block: bool = false;

        config_client.send(self.stream_config.clone()).await?;

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
                            tracing::info!("[🔍 Indexer] 🥳🎉 Reached pending block!");
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

        // Create the new position & sends it to the monitoring service.
        if let Some(new_position) = Position::from_event(&self.config, &event.keys) {
            let position_key = new_position.key();
            if self.seen_positions.insert(position_key) {
                tracing::info!(
                    "[🔍 Indexer] Found new position 0x{:x} at block {}",
                    new_position.key(),
                    block_number
                );
            }
            match self.positions_sender.try_send((block_number, new_position)) {
                Ok(_) => {}
                Err(e) => panic!("[🔍 Indexer] 😱 Could not send position: {}", e),
            }
        }
        Ok(())
    }
}
