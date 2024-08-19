use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::sync::Mutex;

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

// Thread-safe wrapper around HashSet
pub struct Positions(Arc<Mutex<HashSet<Position>>>);

impl Positions {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashSet::new())))
    }

    pub async fn insert(&self, position: Position) -> bool {
        self.0.lock().await.insert(position)
    }

    pub async fn is_empty(&self) -> bool {
        self.0.lock().await.is_empty()
    }

    pub async fn drain(&self) -> Vec<Position> {
        self.0.lock().await.drain().collect()
    }

    pub async fn len(&self) -> usize {
        self.0.lock().await.len()
    }
}

impl Default for Positions {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MonitoringService {
    pub rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    pub pragma_oracle: Arc<PragmaOracle>,
    pub position_receiver: Receiver<Position>,
    pub positions: Positions,
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
            positions: Positions::new(),
        }
    }

    pub async fn start(mut self) {
        let mut update_interval = interval(Duration::from_secs(CHECK_POSITIONS_INTERVAL));

        loop {
            tokio::select! {
                _ = update_interval.tick() => {
                    self.update_and_monitor_health().await;
                }
                maybe_position = self.position_receiver.recv() => {
                    match maybe_position {
                        Some(position) => {
                            if self.positions.insert(position).await {
                                self.update_and_monitor_health().await;
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

    /// Update all monitored positions and check if it's worth to liquidate any.
    /// TODO: Check issue for multicall update:
    /// https://github.com/astraly-labs/vesu-liquidator/issues/12
    async fn update_and_monitor_health(&self) {
        println!("🔎 Checking if any position is liquidable...");
        if self.positions.is_empty().await {
            return;
        }
        self.update_all_positions().await;
        // TODO: get LTV
        // TODO: check health
        // TODO: check if worth to liquidate if liquidable
        // TODO: liquidate
        println!("🤨 They're good.. for now...");
    }

    /// Update all monitored positions
    async fn update_all_positions(&self) {
        if self.positions.is_empty().await {
            return;
        }

        let positions = self.positions.drain().await;
        let updated_positions = Positions::new();

        for position in positions {
            let updated_position = self.update_position(position).await;
            updated_positions.insert(updated_position).await;
        }

        *self.positions.0.lock().await = updated_positions.0.lock().await.clone();
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
