use std::{collections::HashSet, sync::Arc, time::Duration};

use bigdecimal::BigDecimal;
use starknet::{
    core::types::{BlockId, BlockTag, FunctionCall},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
};

use tokio::sync::mpsc::Receiver;
use tokio::time::interval;
use url::Url;

use crate::{
    config::{VESU_POSITION_UNSAFE_SELECTOR, VESU_SINGLETON_CONTRACT},
    oracle::PragmaOracle,
    types::Position,
};

// TODO: Should be a CLI arg
const CHECK_POSITIONS_INTERVAL: u64 = 10;

pub struct MonitoringService {
    pub rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    pub pragma_oracle: Arc<PragmaOracle>,
    pub position_receiver: Receiver<Position>,
    pub positions: HashSet<Position>,
}

impl MonitoringService {
    pub fn new(
        rpc_url: Url,
        pragma_api_key: String,
        position_receiver: Receiver<Position>,
    ) -> MonitoringService {
        MonitoringService {
            rpc_client: Arc::new(JsonRpcClient::new(HttpTransport::new(rpc_url))),
            pragma_oracle: Arc::new(PragmaOracle::new(pragma_api_key)),
            position_receiver,
            positions: HashSet::default(),
        }
    }

    pub async fn start(mut self) {
        let mut update_interval = interval(Duration::from_secs(CHECK_POSITIONS_INTERVAL));

        loop {
            tokio::select! {
                _ = update_interval.tick() => {
                    if !self.positions.is_empty() {
                        println!("🔎 Checking if any position is liquidable...");
                        self.update_all_positions().await;
                        println!("🤨 They're good.. for now...");
                        // TODO: handle liquidations...
                    }
                }
                position = self.position_receiver.recv() => {
                    match position {
                        Some(position) => {
                            if self.positions.insert(position) {
                                println!("🥡 New position received! Monitoring {} positions...", self.positions.len());
                            }
                        }
                        None => {
                            println!("Position channel closed, exiting.");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Update all monitored positions
    async fn update_all_positions(&mut self) {
        if self.positions.is_empty() {
            return;
        }

        let positions: Vec<Position> = self.positions.drain().collect();
        let mut updated_positions = HashSet::new();

        for position in positions {
            let updated_position = self.update_position(position).await;
            updated_positions.insert(updated_position);
        }

        self.positions = updated_positions;
    }

    /// Update a position given the latest data available.
    async fn update_position(&self, mut position: Position) -> Position {
        let get_position_request = &FunctionCall {
            contract_address: VESU_SINGLETON_CONTRACT.to_owned(),
            entry_point_selector: VESU_POSITION_UNSAFE_SELECTOR.to_owned(),
            calldata: position.as_calldata(),
        };
        let result = self
            .rpc_client
            .call(get_position_request, BlockId::Tag(BlockTag::Pending))
            .await
            .expect("failed to request position state");

        position.collateral.amount =
            BigDecimal::new(result[4].to_bigint(), position.collateral.decimals);
        position.debt.amount = BigDecimal::new(result[6].to_bigint(), position.debt.decimals);
        position
    }
}
