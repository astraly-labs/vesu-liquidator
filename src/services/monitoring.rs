use std::{sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use dashmap::DashSet;
use futures_util::lock::Mutex;
use starknet_rust::core::types::Felt;
use starknet_rust::providers::{JsonRpcClient, jsonrpc::HttpTransport};
use tokio::task::JoinSet;
use tokio::{sync::mpsc::UnboundedReceiver, time::interval};

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

/// How often we scan all positions for liquidation opportunities.
const CHECK_POSITIONS_INTERVAL_MS: u64 = 500;

/// Max attempts to update a position before giving up (don't block the loop forever).
const MAX_POSITION_UPDATE_ATTEMPTS: u32 = 3;

/// Positions we're currently liquidating — avoid double-firing.
type InFlightSet = Arc<DashSet<u64>>;

#[derive(Clone)]
pub struct MonitoringService {
    liquidate_address: Felt,
    config: Config,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    account: Arc<StarknetAccount>,
    positions_receiver: Arc<Mutex<UnboundedReceiver<(u64, Position)>>>,
    positions: PositionsMap,
    latest_oracle_prices: LatestOraclePrices,
    storage: Arc<Mutex<Box<dyn Storage>>>,
    http_client: reqwest::Client,
    in_flight: InFlightSet,
}

#[async_trait::async_trait]
impl Service for MonitoringService {
    async fn start(&mut self, join_set: &mut JoinSet<anyhow::Result<()>>) -> anyhow::Result<()> {
        let service = self.clone();
        join_set.spawn(async move {
            tracing::info!("🔭 Monitoring service started");
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
        positions_receiver: UnboundedReceiver<(u64, Position)>,
        latest_oracle_prices: LatestOraclePrices,
        storage: Box<dyn Storage>,
    ) -> MonitoringService {
        MonitoringService {
            liquidate_address: config.liquidate_address,
            config,
            rpc_client,
            account: Arc::new(account),
            positions_receiver: Arc::new(Mutex::new(positions_receiver)),
            positions: PositionsMap::from_storage(storage.as_ref()),
            latest_oracle_prices,
            storage: Arc::new(Mutex::new(storage)),
            http_client: reqwest::Client::new(),
            in_flight: Arc::new(DashSet::new()),
        }
    }

    pub async fn run_forever(&self) -> Result<()> {
        let mut scan_interval = interval(Duration::from_millis(CHECK_POSITIONS_INTERVAL_MS));

        loop {
            let mut receiver = self.positions_receiver.lock().await;

            tokio::select! {
                // Tight scan loop — check all positions every 500ms
                _ = scan_interval.tick() => {
                    drop(receiver);
                    self.scan_and_liquidate().await;
                }

                // Ingest new positions from indexer
                maybe_position = receiver.recv() => {
                    drop(receiver);
                    match maybe_position {
                        Some((block_number, mut new_position)) => {
                            if let Err(e) = new_position
                                .try_update(&self.rpc_client, &self.config.singleton_address, MAX_POSITION_UPDATE_ATTEMPTS)
                                .await
                            {
                                tracing::warn!("[🔭 Monitoring] Could not update new position: {e}");
                                continue;
                            }
                            if new_position.is_closed() {
                                continue;
                            }
                            self.positions.0.insert(new_position.key(), new_position);
                            // Save in background — don't block the hot path
                            let storage = Arc::clone(&self.storage);
                            let positions = self.positions.0.clone();
                            tokio::spawn(async move {
                                if let Err(e) = storage.lock().await.save(&positions, block_number).await {
                                    tracing::warn!("[🔭 Monitoring] Background save failed: {e}");
                                }
                            });
                        }
                        None => {
                            return Err(anyhow!("Monitoring stopped unexpectedly"));
                        }
                    }
                }
            }
        }
    }

    /// Scan all positions and fire liquidations concurrently. Non-blocking.
    async fn scan_and_liquidate(&self) {
        if self.positions.0.is_empty() {
            return;
        }

        let position_keys: Vec<u64> = self.positions.0.iter().map(|entry| *entry.key()).collect();
        let positions_to_delete: Vec<u64> = vec![];

        for key in position_keys {
            // Skip if already being liquidated
            if self.in_flight.contains(&key) {
                continue;
            }

            if let Some(entry) = self.positions.0.get(&key) {
                let position = entry.value().clone();

                let is_liquidable = match position.is_liquidable(&self.latest_oracle_prices).await {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if !is_liquidable {
                    continue;
                }

                tracing::info!(
                    "[🔭 Monitoring] Liquidatable position found #{}! Firing TX...",
                    position.key()
                );

                // Mark as in-flight and fire-and-forget
                self.in_flight.insert(key);
                let this = self.clone();
                tokio::spawn(async move {
                    this.execute_liquidation(key, &position).await;
                    this.in_flight.remove(&key);
                });
            }
        }

        for to_delete in positions_to_delete {
            self.positions.0.remove(&to_delete);
        }
    }

    /// Execute a single liquidation. Runs in a spawned task.
    async fn execute_liquidation(&self, key: u64, position: &Position) {
        let started_at = std::time::Instant::now();

        let liquidation_tx = match position
            .get_vesu_liquidate_tx(
                &self.liquidate_address,
                &self.http_client,
                &self.account.account_address(),
            )
            .await
        {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("[🔭 Monitoring] Failed to build liquidation TX for #{key:x}: {e}");
                return;
            }
        };

        let tx_hash = match self.account.execute_txs(&[liquidation_tx]).await {
            Ok(h) => h,
            Err(e) => {
                if e.to_string().contains("not-undercollateralized") {
                    tracing::warn!(
                        "[🔭 Monitoring] Position #{key:x} was not undercollateralized (race lost)"
                    );
                    self.positions.0.remove(&key);
                } else {
                    tracing::error!("[🔭 Monitoring] TX send failed for #{key:x}: {e}");
                }
                return;
            }
        };

        tracing::info!(
            "[🔭 Monitoring] TX sent for #{key:x}: {tx_hash:#064x} - build took {:?}",
            started_at.elapsed()
        );

        // Wait for confirmation in background — don't block other liquidations
        match wait_for_tx(&self.rpc_client, tx_hash).await {
            Ok(_) => {
                tracing::info!(
                    "[🔭 Monitoring] ✅ Liquidated #{key:x}! (tx {tx_hash:#064x}) - total {:?}",
                    started_at.elapsed()
                );
                // Refresh position after liquidation
                if let Some(mut entry) = self.positions.0.get_mut(&key) {
                    let _ = entry
                        .value_mut()
                        .try_update(&self.rpc_client, &self.config.singleton_address, 2)
                        .await;
                }
            }
            Err(e) => {
                tracing::error!("[🔭 Monitoring] TX {tx_hash:#064x} failed: {e}");
            }
        }
    }
}
