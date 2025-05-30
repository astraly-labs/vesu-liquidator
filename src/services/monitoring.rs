use std::{sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use futures_util::lock::Mutex;
use starknet::providers::{JsonRpcClient, jsonrpc::HttpTransport};
use tokio::task::JoinSet;
use tokio::{
    sync::mpsc::UnboundedReceiver,
    time::{interval, sleep},
};

use crate::bindings::liquidate::Liquidate;
use crate::types::StarknetSingleOwnerAccount;
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

#[derive(Clone)]
pub struct MonitoringService {
    liquidate_contract: Arc<Liquidate<StarknetSingleOwnerAccount>>,
    config: Config,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    account: Arc<StarknetAccount>,
    positions_receiver: Arc<Mutex<UnboundedReceiver<(u64, Position)>>>,
    positions: PositionsMap,
    latest_oracle_prices: LatestOraclePrices,
    storage: Arc<Mutex<Box<dyn Storage>>>,
    http_client: reqwest::Client,
}

#[async_trait::async_trait]
impl Service for MonitoringService {
    async fn start(&mut self, join_set: &mut JoinSet<anyhow::Result<()>>) -> anyhow::Result<()> {
        let service = self.clone();
        // We wait a few seconds before starting the monitoring service to be sure that we have prices
        // + indexed a few positions.
        sleep(Duration::from_secs(4)).await;
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
            liquidate_contract: Arc::new(Liquidate::new(
                config.liquidate_address,
                account.0.clone(),
            )),
            config,
            rpc_client,
            account: Arc::new(account),
            positions_receiver: Arc::new(Mutex::new(positions_receiver)),
            positions: PositionsMap::from_storage(storage.as_ref()),
            latest_oracle_prices,
            storage: Arc::new(Mutex::new(storage)),
            http_client: reqwest::Client::new(),
        }
    }

    /// Starts the monitoring service.
    pub async fn run_forever(&self) -> Result<()> {
        const CHECK_POSITIONS_INTERVAL: u64 = 3500;
        let mut update_interval = interval(Duration::from_millis(CHECK_POSITIONS_INTERVAL));

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

        let position_keys: Vec<u64> = self.positions.0.iter().map(|entry| *entry.key()).collect();
        let mut positions_to_delete = vec![];

        for key in position_keys {
            if let Some(mut entry) = self.positions.0.get_mut(&key) {
                let position = entry.value_mut();

                if !position.is_liquidable(&self.latest_oracle_prices).await? {
                    continue;
                }
                tracing::info!(
                    "[🔭 Monitoring] Liquidatable position found #{}!",
                    position.key()
                );

                tracing::info!("[🔭 Monitoring] 🔫 Liquidating position...");
                if let Err(e) = self.liquidate_position(position).await {
                    if e.to_string().contains("not-undercollateralized") {
                        tracing::warn!("[🔭 Monitoring] Position was not under collateralized!");
                        positions_to_delete.push(key);
                        continue;
                    } else {
                        tracing::error!(
                            error = %e,
                            "[🔭 Monitoring] 😨 Could not liquidate position #{:x}",
                            position.key(),
                        );
                    }
                }

                position
                    .update(&self.rpc_client, &self.config.singleton_address)
                    .await?;
            }
        }

        for to_delete in positions_to_delete {
            self.positions.0.remove(&to_delete);
        }

        Ok(())
    }

    /// Check if a position is liquidable, computes the profitability and if it's worth it
    /// liquidate it.
    async fn liquidate_position(&self, position: &Position) -> Result<()> {
        let started_at = std::time::Instant::now();
        let liquidation_tx = position
            .get_vesu_liquidate_tx(
                &self.liquidate_contract,
                &self.http_client,
                &self.account.account_address(),
            )
            .await?;
        let tx_hash = self.account.execute_txs(&[liquidation_tx]).await?;
        wait_for_tx(&self.rpc_client, tx_hash).await?;
        tracing::info!(
            "[🔭 Monitoring] ✅ Liquidated position #{}! (tx {tx_hash:#064x}) - ⌛ {:?}",
            position.key(),
            started_at.elapsed()
        );
        Ok(())
    }
}
