use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use bigdecimal::BigDecimal;
use dashmap::DashMap;
use futures_util::future::join_all;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use strum::Display;
use tokio::task::JoinSet;
use url::Url;

use crate::cli::NetworkName;
use crate::utils::services::Service;
use crate::{config::Config, utils::conversions::hex_str_to_big_decimal};

const USD_ASSET: &str = "usd";
const PRICES_UPDATE_INTERVAL: u64 = 30; // update every 30 seconds

#[derive(Clone)]
pub struct OracleService {
    oracle: PragmaOracle,
    latest_prices: LatestOraclePrices,
}

#[async_trait::async_trait]
impl Service for OracleService {
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

impl OracleService {
    pub fn new(
        api_url: Url,
        api_key: String,
        latest_prices: LatestOraclePrices,
        network: NetworkName,
    ) -> Self {
        let network_to_fetch = match network {
            NetworkName::Sepolia => "sepolia",
            NetworkName::Mainnet => "mainnet",
        };
        let oracle = PragmaOracle::new(api_url, api_key, network_to_fetch.to_string());
        Self {
            oracle,
            latest_prices,
        }
    }

    /// Starts the oracle service that will fetch the latest oracle prices every
    /// PRICES_UPDATE_INTERVAL seconds.
    pub async fn run_forever(self) -> Result<()> {
        let sleep_duration = Duration::from_secs(PRICES_UPDATE_INTERVAL);
        loop {
            tracing::info!("[🔮 Oracle] Fetching latest prices...");
            self.update_prices().await?;
            tracing::info!("[🔮 Oracle] ✅ Fetched all new prices");
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

        let fetch_tasks = assets.into_iter().map(|asset| {
            let oracle = self.oracle.clone();
            async move {
                let price = oracle.get_dollar_price(asset.clone()).await;
                (asset, price)
            }
        });

        let results = join_all(fetch_tasks).await;

        for (asset, price_result) in results {
            if let Ok(price) = price_result {
                self.latest_prices.0.insert(asset, price);
            }
        }

        Ok(())
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

#[derive(Deserialize, Debug)]
pub struct OracleApiResponse {
    pub price: String,
    pub decimals: i64,
}

#[derive(Debug, Clone)]
pub struct PragmaOracle {
    http_client: reqwest::Client,
    api_url: Url,
    api_key: String,
    aggregation_method: AggregationMethod,
    interval: Interval,
    network: String,
}

impl PragmaOracle {
    pub fn new(api_url: Url, api_key: String, network: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            api_url,
            api_key,
            aggregation_method: AggregationMethod::Median,
            interval: Interval::OneMinute,
            network,
        }
    }
}

impl PragmaOracle {
    // TODO: Fix oracle timeout response with a retry
    pub async fn get_dollar_price(&self, asset_name: String) -> Result<BigDecimal> {
        let url = self.fetch_price_url(asset_name, USD_ASSET.to_owned());
        let response = self
            .http_client
            .get(url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?;
        let response_status = response.status();
        let response_text = response.text().await?;
        if response_status != StatusCode::OK {
            tracing::error!("⛔ Oracle Request failed with: {:?}", response_text);
            return Err(anyhow!(
                "Oracle request failed with status {response_status}"
            ));
        }
        let oracle_response: OracleApiResponse = serde_json::from_str(&response_text)?;
        let asset_price = hex_str_to_big_decimal(&oracle_response.price, oracle_response.decimals);
        Ok(asset_price)
    }

    fn fetch_price_url(&self, base: String, quote: String) -> String {
        format!(
            "{}node/v1/onchain/{}/{}?network={}&components=false&variations=false&interval={}&aggregation={}",
            self.api_url, base, quote, self.network, self.interval, self.aggregation_method
        )
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, Display)]
/// Supported Aggregation Methods
pub enum AggregationMethod {
    #[serde(rename = "median")]
    #[strum(serialize = "median")]
    #[default]
    Median,
    #[serde(rename = "mean")]
    #[strum(serialize = "mean")]
    Mean,
    #[strum(serialize = "twap")]
    #[serde(rename = "twap")]
    Twap,
}

/// Supported Aggregation Intervals
#[derive(Default, Debug, Serialize, Deserialize, Clone, Display)]
pub enum Interval {
    #[serde(rename = "1min")]
    #[strum(serialize = "1min")]
    OneMinute,
    #[serde(rename = "15min")]
    #[strum(serialize = "15min")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    #[strum(serialize = "1h")]
    OneHour,
    #[serde(rename = "2h")]
    #[strum(serialize = "2h")]
    #[default]
    TwoHours,
}
