use crate::{
    config::{MODIFY_POSITION_EVENT, VESU_SINGLETON_CONTRACT},
    types::position::Position,
    utils::conversions::{apibara_field_element_as_felt, felt_as_apibara_field_element},
};
use apibara_core::{
    node::v1alpha2::DataFinality,
    starknet::v1alpha2::{Block, Filter, HeaderFilter},
};
use apibara_sdk::{configuration, ClientBuilder, Configuration, Uri};
use futures_util::TryStreamExt;
use starknet::core::types::Felt;
use tokio::sync::mpsc::Sender;

// Constants for execution.
// Only for testing purposes.
// TODO: Should be CLI args.
const FROM_BLOCK: u64 = 668_220;

const INDEXING_STREAM_CHUNK_SIZE: usize = 128;

pub struct IndexerService {
    uri: Uri,
    apibara_api_key: String,
    stream_config: Configuration<Filter>,
    positions_sender: Sender<Position>,
}

impl IndexerService {
    pub fn new(apibara_api_key: String, positions_sender: Sender<Position>) -> IndexerService {
        // TODO: change if sepolia to https://sepolia.starknet.a5a.ch
        let uri: Uri = Uri::from_static("https://mainnet.starknet.a5a.ch");

        let stream_config = Configuration::<Filter>::default()
            .with_starting_block(FROM_BLOCK)
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
            uri,
            apibara_api_key,
            stream_config,
            positions_sender,
        }
    }

    /// Retrieve all the ModifyPosition events emitted from the Vesu Singleton Contract.
    pub async fn start(self) -> ! {
        let (config_client, config_stream) = configuration::channel(INDEXING_STREAM_CHUNK_SIZE);
        config_client.send(self.stream_config).await.unwrap();

        let mut stream = ClientBuilder::default()
            .with_bearer_token(Some(self.apibara_api_key))
            .connect(self.uri)
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
                                    let new_position =
                                        Position::try_from_event(&event.keys).unwrap();
                                    let _ = self.positions_sender.try_send(new_position);
                                }
                            }
                        }
                    }
                    apibara_sdk::DataMessage::Invalidate { cursor } => match cursor {
                        Some(c) => {
                            // TODO: don't panic, handle error
                            panic!("Received an invalidate request data at {}", &c.order_key)
                        }
                        // TODO: don't panic, handle error
                        None => panic!("Invalidate request without cursor provided"),
                    },
                    apibara_sdk::DataMessage::Heartbeat => {
                        println!("🥰")
                    }
                },
                Ok(None) => continue,
                Err(e) => {
                    // TODO: don't panic, handle error
                    panic!("Error while streaming: {}", e);
                }
            }
        }
    }
}
