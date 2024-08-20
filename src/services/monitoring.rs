use anyhow::Result;
use bigdecimal::BigDecimal;
use colored::Colorize;
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;

use tokio::sync::mpsc::Receiver;
use tokio::time::interval;

use crate::{oracle::PragmaOracle, types::position::Position};

// TODO: Should be a CLI arg
const CHECK_POSITIONS_INTERVAL: u64 = 10;

// Thread-safe wrapper around HashSet
pub struct Positions(Arc<RwLock<HashMap<u64, Position>>>);

impl Positions {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    pub async fn insert(&self, position: Position) -> Option<Position> {
        self.0.write().await.insert(position.key(), position)
    }

    pub async fn is_empty(&self) -> bool {
        self.0.read().await.is_empty()
    }

    pub async fn len(&self) -> usize {
        self.0.read().await.len()
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
        rpc_client: Arc<JsonRpcClient<HttpTransport>>,
        pragma_api_key: String,
        position_receiver: Receiver<Position>,
    ) -> MonitoringService {
        MonitoringService {
            rpc_client,
            pragma_oracle: Arc::new(PragmaOracle::new(pragma_api_key)),
            position_receiver,
            positions: Positions::new(),
        }
    }

    pub async fn start(mut self) -> Result<()> {
        let mut update_interval = interval(Duration::from_secs(CHECK_POSITIONS_INTERVAL));

        loop {
            tokio::select! {
                _ = update_interval.tick() => {
                    self.monitor_positions_health().await;
                }
                maybe_position = self.position_receiver.recv() => {
                    match maybe_position {
                        Some(position) => {
                            self.positions.insert(position).await;
                            // Flush all pending positions in one go...
                            while let Ok(position) = self.position_receiver.try_recv() {
                                self.positions.insert(position).await;
                            }
                        }
                        None => {
                            return Err(anyhow::anyhow!("Monitoring stopped unexpectedly."));
                        }
                    }
                }
            }
        }
    }

    /// Update all monitored positions and check if it's worth to liquidate any.
    /// TODO: Check issue for multicall update:
    /// https://github.com/astraly-labs/vesu-liquidator/issues/12
    async fn monitor_positions_health(&self) {
        if self.positions.is_empty().await {
            return;
        }

        println!("\n🔎 Checking if any position is liquidable...");
        for (_, position) in self.positions.0.read().await.iter() {
            if position.is_closed() {
                continue;
            }
            self.is_liquidable(position).await;
        }

        // TODO: check if worth to liquidate if liquidable (compute_profitability)
        // TODO: liquidate

        println!("🤨 They're good.. for now...");
    }

    async fn is_liquidable(&self, position: &Position) -> bool {
        let ltv_ratio = position
            .ltv(&self.pragma_oracle)
            .await
            .expect("failed to retrieve ltv ratio");

        let result = ltv_ratio > position.lltv;
        println!(
            "Position {}/{} of user {:?} is curently at ratio {:.2}%/{:.2}% => {}",
            position.collateral.name,
            position.debt.name,
            position.user_address,
            ltv_ratio * 100,
            position.lltv.clone() * 100,
            if result {
                "is liquidable".green()
            } else {
                "is NOT liquidable".red()
            }
        );
        result
    }

    #[allow(unused)]
    // TODO: compute profitability after simulation
    async fn compute_profitability(&self, position: Position) -> BigDecimal {
        let max_debt_in_dollar = position.collateral.amount
            * position.lltv
            * self
                .pragma_oracle
                .get_dollar_price(position.collateral.name)
                .await
                .unwrap();
        let current_debt = position.debt.amount
            * self
                .pragma_oracle
                .get_dollar_price(position.debt.name)
                .await
                .unwrap();
        // +1 to be slighly under threshold
        let amount_to_liquidate = current_debt - (max_debt_in_dollar + 1);

        // TODO : get flashloan fees
        let flashloan_fees = BigDecimal::from(0);

        amount_to_liquidate - flashloan_fees
    }
}
