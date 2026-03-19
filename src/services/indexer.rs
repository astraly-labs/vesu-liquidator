use anyhow::Result;
use apibara_dna_protocol::dna::stream::DataFinality;
use apibara_dna_sdk::{
    StaticBearerToken, StreamClient, StreamDataRequestBuilder,
    proto::{Cursor, DnaMessage},
    starknet::{Block, Event, EventFilterBuilder, FieldElement, FilterBuilder},
};
use dashmap::DashSet;
use prost::Message;
use starknet_rust::core::types::Felt;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinSet;
use tokio_stream::StreamExt;
use tonic::transport::Uri;

use crate::cli::NetworkName;
use crate::config::{Config, MIGRATE_POSITION_EVENT, MODIFY_POSITION_EVENT};
use crate::types::position::Position;
use crate::utils::conversions::{apibara_field_as_felt, felt_as_apibara_field};
use crate::utils::services::Service;

const STARKNET_MAINNET_DNA_URL: &str = "https://mainnet.starknet.a5a.ch";
const STARKNET_SEPOLIA_DNA_URL: &str = "https://sepolia.starknet.a5a.ch";

#[derive(Clone)]
pub struct IndexerService {
    config: Config,
    dna_url: String,
    apibara_api_key: String,
    starting_block: u64,
    positions_sender: UnboundedSender<(u64, Position)>,
    seen_positions: DashSet<u64>,
}

#[async_trait::async_trait]
impl Service for IndexerService {
    async fn start(&mut self, join_set: &mut JoinSet<anyhow::Result<()>>) -> anyhow::Result<()> {
        let service = self.clone();
        join_set.spawn(async move {
            tracing::info!("🔍 Indexer service started");
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
        positions_sender: UnboundedSender<(u64, Position)>,
        from_block: u64,
    ) -> IndexerService {
        let dna_url = match config.network {
            NetworkName::Mainnet => STARKNET_MAINNET_DNA_URL,
            NetworkName::Sepolia => STARKNET_SEPOLIA_DNA_URL,
        };

        IndexerService {
            config,
            dna_url: dna_url.to_string(),
            apibara_api_key,
            starting_block: from_block,
            positions_sender,
            seen_positions: DashSet::default(),
        }
    }

    fn build_filter(&self) -> apibara_dna_sdk::starknet::Filter {
        let singleton = felt_as_apibara_field(&self.config.singleton_address);
        let modify_key = felt_as_apibara_field(&MODIFY_POSITION_EVENT);
        let migrate_key = felt_as_apibara_field(&MIGRATE_POSITION_EVENT);

        FilterBuilder::new()
            .add_event(
                EventFilterBuilder::single_contract(singleton)
                    .with_keys(false, vec![Some(modify_key)])
                    .build(),
            )
            .add_event(
                EventFilterBuilder::single_contract(singleton)
                    .with_keys(false, vec![Some(migrate_key)])
                    .build(),
            )
            .build()
    }

    pub async fn run_forever(mut self) -> Result<()> {
        let filter = self.build_filter();

        let stream_request = StreamDataRequestBuilder::new()
            .with_starting_cursor(Cursor::new_with_block_number(self.starting_block))
            .with_finality(DataFinality::Pending)
            .add_filter(filter)
            .build();

        let url: Uri = self.dna_url.parse()?;

        let mut client = StreamClient::builder()
            .with_bearer_token_provider(Arc::new(StaticBearerToken::new(
                self.apibara_api_key.clone(),
            )))
            .connect(url)
            .await
            .map_err(|e| anyhow::anyhow!("Could not connect to Apibara DNA: {e}"))?;

        let mut stream = client
            .stream_data(stream_request)
            .await
            .map_err(|e| anyhow::anyhow!("Could not start Apibara DNA stream: {e}"))?;

        let mut reached_live: bool = false;

        loop {
            match stream.try_next().await {
                Ok(Some(msg)) => match msg {
                    DnaMessage::Data(data) => {
                        const DATA_PRODUCTION_LIVE: i32 = 2;
                        if !reached_live && data.production == DATA_PRODUCTION_LIVE {
                            tracing::info!("[🔍 Indexer] 🥳🎉 Reached pending block!");
                            reached_live = true;
                        }

                        for block_bytes in data.data {
                            let block = Block::decode(block_bytes)?;
                            let block_number =
                                block.header.as_ref().map(|h| h.block_number).unwrap_or(0);

                            for event in block.events {
                                self.create_position_from_event(block_number, event).await?;
                            }
                        }
                    }
                    DnaMessage::Invalidate(invalidated) => {
                        if let Some(cursor) = invalidated.cursor {
                            tracing::warn!(
                                "[🔍 Indexer] Received invalidate at block {}",
                                cursor.order_key
                            );
                        }
                    }
                    DnaMessage::Finalize(_)
                    | DnaMessage::Heartbeat(_)
                    | DnaMessage::SystemMessage(_) => {}
                },
                Ok(None) => continue,
                Err(e) => {
                    tracing::error!("[🔍 Indexer] Error while streaming: {}", e);
                }
            }
        }
    }

    async fn create_position_from_event(&mut self, block_number: u64, event: Event) -> Result<()> {
        if event.from_address.is_none() {
            return Ok(());
        }

        let keys: Vec<FieldElement> = event.keys.clone();
        if keys.len() < 4 {
            return Ok(());
        }

        let debt_address = apibara_field_as_felt(&keys[3]);
        // Events from the extension contract have debt_address == 0 — ignore them.
        if debt_address == Felt::ZERO {
            return Ok(());
        }

        if let Some(new_position) = Position::from_event(&self.config, &keys) {
            let position_key = new_position.key();
            if self.seen_positions.insert(position_key) {
                tracing::info!(
                    "[🔍 Indexer] Found new/updated position at block {}",
                    block_number
                );
            }
            if let Err(e) = self.positions_sender.send((block_number, new_position)) {
                panic!("[🔍 Indexer] 😱 Could not send position: {}", e);
            }
        } else {
            tracing::error!("Could not create position from event :/");
        }
        Ok(())
    }
}
