use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use bigdecimal::BigDecimal;
use dashmap::DashMap;
use futures_util::future::join_all;
use starknet::core::types::{BlockId, BlockTag, Felt, FunctionCall};
use starknet::core::utils::{cairo_short_string_to_felt, get_selector_from_name};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider};
use tokio::task::JoinSet;

use crate::config::Config;
use crate::utils::conversions::hex_str_to_big_decimal;
use crate::utils::services::Service;

const LST_ASSETS: [&str; 3] = ["xstrk", "sstrk", "kstrk"];

/// Aggregations possible using the Pragma Oracle contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationMode {
    Median,
    Mean,
    ConversionRate,
}

impl AggregationMode {
    pub fn to_felt(&self) -> Felt {
        match self {
            AggregationMode::Median => Felt::ZERO,
            AggregationMode::Mean => Felt::ONE,
            AggregationMode::ConversionRate => Felt::TWO,
        }
    }
}

/// Map contaning the price in dollars for a list of monitored assets.
#[derive(Default, Clone)]
pub struct LatestOraclePrices(pub Arc<DashMap<String, BigDecimal>>);

impl LatestOraclePrices {
    pub fn from_config(config: &Config) -> Self {
        let prices = DashMap::new();
        for asset in config.assets.iter() {
            prices.insert(asset.ticker.to_lowercase(), BigDecimal::default());
        }
        LatestOraclePrices(Arc::new(prices))
    }
}

#[derive(Clone)]
pub struct OracleService {
    pragma_address: Felt,
    rpc_client: Arc<JsonRpcClient<HttpTransport>>,
    latest_prices: LatestOraclePrices,
}

#[async_trait::async_trait]
impl Service for OracleService {
    async fn start(&mut self, join_set: &mut JoinSet<anyhow::Result<()>>) -> anyhow::Result<()> {
        let service = self.clone();
        join_set.spawn(async move {
            tracing::info!("🔮 Oracle service started");
            service.run_forever().await?;
            Ok(())
        });
        Ok(())
    }
}

impl OracleService {
    pub fn new(
        pragma_address: Felt,
        rpc_client: Arc<JsonRpcClient<HttpTransport>>,
        latest_prices: LatestOraclePrices,
    ) -> Self {
        Self {
            pragma_address,
            rpc_client,
            latest_prices,
        }
    }

    /// Starts the oracle service that will fetch the latest oracle prices every
    /// PRICES_UPDATE_INTERVAL seconds.
    pub async fn run_forever(self) -> Result<()> {
        const PRICES_UPDATE_INTERVAL: u64 = 3;
        let sleep_duration = Duration::from_secs(PRICES_UPDATE_INTERVAL);
        loop {
            self.update_prices().await?;
            tokio::time::sleep(sleep_duration).await;
        }
    }

    /// Update all the monitored assets with their latest USD price asynchronously.
    async fn update_prices(&self) -> Result<()> {
        let assets: Vec<String> = self
            .latest_prices
            .0
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        let fetch_tasks = assets.into_iter().map(|asset| async move {
            let price = self.get_price_in_dollars(&asset).await;
            (asset, price)
        });

        let results = join_all(fetch_tasks).await;

        for (asset, price_result) in results {
            if let Ok(price) = price_result {
                self.latest_prices.0.insert(asset, price);
            }
        }

        Ok(())
    }

    async fn get_price_in_dollars(&self, base_asset: &str) -> Result<BigDecimal> {
        let pair = format!("{}/USD", base_asset.to_ascii_uppercase());

        let aggregation_mode = if LST_ASSETS.contains(&base_asset) {
            AggregationMode::ConversionRate
        } else {
            AggregationMode::Median
        };

        let price_request = FunctionCall {
            contract_address: self.pragma_address,
            entry_point_selector: get_selector_from_name("get_data")?,
            calldata: vec![
                Felt::ZERO,
                cairo_short_string_to_felt(&pair)?,
                aggregation_mode.to_felt(),
            ],
        };

        let call_result = self
            .rpc_client
            .call(price_request, BlockId::Tag(BlockTag::PreConfirmed))
            .await?;

        let asset_price = hex_str_to_big_decimal(
            &call_result[0].to_hex_string(),
            call_result[1].to_bigint().try_into()?,
        );

        Ok(asset_price)
    }
}
