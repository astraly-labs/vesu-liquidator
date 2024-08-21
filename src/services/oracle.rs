use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use strum::Display;
use tokio::sync::Mutex;
use url::Url;

use crate::{config::Config, utils::conversions::hexa_price_to_big_decimal};

pub const USD_ASSET: &str = "usd";
pub const PRICES_UPDATE_INTERVAL: u64 = 55;

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

    pub async fn start(self) -> Result<()> {
        let sleep_duration = Duration::from_secs(PRICES_UPDATE_INTERVAL);
        loop {
            println!("[🔮 Oracle] Fetching latest prices...");
            self.update_prices().await?;
            println!("[🔮 Oracle] Fetched!");
            tokio::time::sleep(sleep_duration).await;
        }
    }

    /// Update all the monitored assets with their latest USD price.
    async fn update_prices(&self) -> Result<()> {
        let mut prices = self.latest_prices.0.lock().await;
        let assets: Vec<String> = prices.keys().cloned().collect();
        for asset in assets {
            if let Ok(price) = self.oracle.get_dollar_price(asset.clone()).await {
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
            prices.insert(asset.ticker.clone(), BigDecimal::default());
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
    pub api_url: Url,
    pub api_key: String,
    pub aggregation_method: AggregationMethod,
    pub interval: Interval,
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
    pub fn fetch_price_url(&self, base: String, quote: String) -> String {
        format!(
            "{}node/v1/data/{}/{}?interval={}&aggregation={}",
            self.api_url, base, quote, self.interval, self.aggregation_method
        )
    }

    // TODO: Fix oracle timeout response with a retry
    pub async fn get_dollar_price(&self, asset_name: String) -> Result<BigDecimal> {
        let url = self.fetch_price_url(asset_name.clone(), USD_ASSET.to_owned());
        let response = self
            .http_client
            .get(url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?;
        if response.status() != 200 {
            println!("⛔ Oracle Request failed with: {:?}", response.text().await);
            panic!("Exiting.");
        }
        let response_text = response.text().await?;
        let oracle_response: OracleApiResponse = serde_json::from_str(&response_text)?;
        Ok(hexa_price_to_big_decimal(
            oracle_response.price.as_str(),
            oracle_response.decimals,
        ))
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
