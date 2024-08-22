use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

use anyhow::{anyhow, Result};
use bigdecimal::BigDecimal;
use futures_util::future::join_all;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use strum::Display;
use tokio::sync::Mutex;
use url::Url;

use crate::{config::Config, utils::conversions::hex_str_to_big_decimal};

const USD_ASSET: &str = "usd";
const PRICES_UPDATE_INTERVAL: u64 = 60; // update every minutes

pub struct OracleService {
    oracle: PragmaOracle,
    latest_prices: LatestOraclePrices,
}

impl OracleService {
    pub fn new(api_url: Url, api_key: String, latest_prices: LatestOraclePrices) -> Self {
        let oracle = PragmaOracle::new(api_url, api_key);
        Self {
            oracle,
            latest_prices,
        }
    }

    /// Starts the oracle service that will fetch the latest oracle prices every
    /// PRICES_UPDATE_INTERVAL seconds.
    pub async fn start(self) -> Result<()> {
        let sleep_duration = Duration::from_secs(PRICES_UPDATE_INTERVAL);
        loop {
            println!("[🔮 Oracle] Fetching latest prices...");
            self.update_prices().await?;
            println!("[🔮 Oracle] ✅ Fetched all new prices");
            tokio::time::sleep(sleep_duration).await;
        }
    }

    /// Update all the monitored assets with their latest USD price asynchronously.
    async fn update_prices(&self) -> Result<()> {
        let mut prices = self.latest_prices.0.lock().await;
        let assets: Vec<String> = prices.keys().cloned().collect();

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
                prices.insert(asset, price);
            }
        }

        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct LatestOraclePrices(pub Arc<Mutex<HashMap<String, BigDecimal>>>);

impl LatestOraclePrices {
    pub fn from_config(config: &Config) -> Self {
        let mut prices = HashMap::new();
        for asset in config.assets.iter() {
            prices.insert(asset.ticker.to_lowercase(), BigDecimal::default());
        }
        LatestOraclePrices(Arc::new(Mutex::new(prices)))
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
}

impl PragmaOracle {
    pub fn new(api_url: Url, api_key: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            api_url,
            api_key,
            aggregation_method: AggregationMethod::Median,
            interval: Interval::OneMinute,
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
            println!("⛔ Oracle Request failed with: {:?}", response_text);
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
            "{}node/v1/onchain/{}/{}?network=sepolia&components=false&variations=false&interval={}&aggregation={}",
            self.api_url, base, quote, self.interval, self.aggregation_method
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
