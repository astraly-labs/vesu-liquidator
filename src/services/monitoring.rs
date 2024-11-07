use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use futures_util::lock::Mutex;
use starknet::{
    core::types::{Call, Felt},
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
};
use tokio::time::{interval, sleep};
use tokio::{sync::mpsc::Receiver, task::JoinSet};

use crate::{
    config::Config,
    services::oracle::LatestOraclePrices,
    storages::Storage,
    types::{
        account::StarknetAccount,
        position::{Position, PositionsMap},
    },
    utils::{services::Service, wait_for_tx},
};

const CHECK_POSITIONS_INTERVAL: u64 = 10;

#[derive(Clone)]
pub struct MonitoringService {
    config: Config,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    account: Arc<StarknetAccount>,
    positions_receiver: Arc<Mutex<Receiver<(u64, Position)>>>,
    positions: PositionsMap,
    latest_oracle_prices: LatestOraclePrices,
    storage: Arc<Mutex<Box<dyn Storage>>>,
}

#[async_trait::async_trait]
impl Service for MonitoringService {
    async fn start(&mut self, join_set: &mut JoinSet<anyhow::Result<()>>) -> anyhow::Result<()> {
        let service = self.clone();
        // We wait a few seconds before starting the monitoring service to be sure that we have prices
        // + indexed a few positions.
        sleep(Duration::from_secs(5)).await;
        join_set.spawn(async move {
            tracing::info!("🧩 Indexer service started");
            service.run_forever().await?;
            Ok(())
        });
        Ok(())
    }
}

impl MonitoringService {
    pub fn new(
        config: Config,
        rpc_client: Arc<JsonRpcClient<HttpTransport>>,
        account: StarknetAccount,
        positions_receiver: Receiver<(u64, Position)>,
        latest_oracle_prices: LatestOraclePrices,
        storage: Box<dyn Storage>,
    ) -> MonitoringService {
        MonitoringService {
            config,
            rpc_client,
            account: Arc::new(account),
            positions_receiver: Arc::new(Mutex::new(positions_receiver)),
            positions: PositionsMap::from_storage(storage.as_ref()),
            latest_oracle_prices,
            storage: Arc::new(Mutex::new(storage)),
        }
    }

    /// Starts the monitoring service.
    pub async fn run_forever(&self) -> Result<()> {
        let mut update_interval = interval(Duration::from_secs(CHECK_POSITIONS_INTERVAL));

        loop {
            let mut receiver = self.positions_receiver.lock().await;

            tokio::select! {
                _ = update_interval.tick() => {
                    drop(receiver);
                    self.monitor_positions_liquidability().await?;
                }

                maybe_position = receiver.recv() => {
                    drop(receiver);
                    match maybe_position {
                        Some((block_number, mut new_position)) => {
                            new_position
                                .update(&self.rpc_client, &self.config.singleton_address)
                                .await?;
                            if new_position.is_closed() {
                                continue;
                            }
                            self.positions.0.insert(new_position.key(), new_position);
                            self.storage.lock().await.save(&self.positions.0, block_number).await?;
                        }
                        None => {
                            return Err(anyhow!("Monitoring stopped unexpectedly"));
                        }
                    }
                }
            }
        }
    }

    /// Update all monitored positions and check if it's worth to liquidate any.
    async fn monitor_positions_liquidability(&self) -> Result<()> {
        if self.positions.0.is_empty() {
            return Ok(());
        }

        tracing::info!("[🔭 Monitoring] Checking if any position is liquidable...");
        let position_keys: Vec<u64> = self.positions.0.iter().map(|entry| *entry.key()).collect();

        for key in position_keys {
            if let Some(mut entry) = self.positions.0.get_mut(&key) {
                let position = entry.value_mut();
                if position.is_liquidable(&self.latest_oracle_prices).await? {
                    tracing::info!(
                        "[🔭 Monitoring] Liquidatable position found #{}!",
                        position.key()
                    );
                    self.liquidate_position(position).await?;
                    position
                        .update(&self.rpc_client, &self.config.singleton_address)
                        .await?;
                }
            }
        }

        tracing::info!("[🔭 Monitoring] 🤨 They're good.. for now...");
        Ok(())
    }

    /// Check if a position is liquidable, computes the profitability and if it's worth it
    /// liquidate it.
    async fn liquidate_position(&self, position: &Position) -> Result<()> {
        let started_at = std::time::Instant::now();
        tracing::info!("[🔭 Monitoring] 🔫 Liquidating position...");
        let tx = self.get_liquidation_tx(position).await?;
        let tx_hash = self.account.execute_txs(&[tx]).await?;
        self.wait_for_tx_to_be_accepted(&tx_hash).await?;
        tracing::info!(
            "[🔭 Monitoring] ✅ Liquidated position #{}! (TX #{}) - ⌛ {:?}",
            position.key(),
            tx_hash.to_hex_string(),
            started_at.elapsed()
        );
        Ok(())
    }

    /// Simulates the profit generated by liquidating a given position. Returns the profit
    /// and the transactions needed to liquidate the position.
    async fn get_liquidation_tx(&self, position: &Position) -> Result<Call> {
        let liquidation_txs = position
            .get_vesu_liquidate_tx(&self.account, self.config.liquidate_address)
            .await?;

        Ok(liquidation_txs)
    }

    /// Waits for a TX to be accepted on-chain.
    pub async fn wait_for_tx_to_be_accepted(&self, &tx_hash: &Felt) -> Result<()> {
        wait_for_tx(tx_hash, &self.rpc_client).await?;
        Ok(())
    }
}
