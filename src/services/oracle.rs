use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

use anyhow::{anyhow, Result};
use bigdecimal::BigDecimal;
use futures_util::{future::join_all, SinkExt, StreamExt};
use regex::Regex;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::Display;
use tokio::sync::Mutex;
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::cli::NetworkName;
use crate::{config::Config, utils::conversions::hex_str_to_big_decimal};

const USD_ASSET: &str = "usd";

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OracleMode {
    Http,
    WebSocket,
}

pub enum OracleServiceMode {
    Http(Duration),
    WebSocket,
}

pub struct OracleService {
    oracle: PragmaOracle,
    latest_prices: LatestOraclePrices,
    mode: OracleServiceMode,
}

impl OracleService {
    pub fn new(
        api_base: String,
        api_key: String,
        latest_prices: LatestOraclePrices,
        network: NetworkName,
        mode: OracleServiceMode,
    ) -> Self {
        let network_to_fetch = match network {
            NetworkName::Sepolia => "sepolia",
            NetworkName::Mainnet => "mainnet",
            #[cfg(feature = "testing")]
            NetworkName::Devnet => "mainnet",
        };
        let oracle = PragmaOracle::new(api_base, api_key, network_to_fetch.to_string());
        Self {
            oracle,
            latest_prices,
            mode,
        }
    }

    pub async fn start(self) -> Result<()> {
        match self.mode {
            OracleServiceMode::Http(update_interval) => self.start_http(update_interval).await,
            OracleServiceMode::WebSocket => self.start_websocket().await,
        }
    }

    async fn start_http(self, update_interval: Duration) -> Result<()> {
        loop {
            tracing::info!("[🔮 Oracle] Fetching latest prices...");
            self.update_prices_http().await?;
            tracing::info!("[🔮 Oracle] ✅ Fetched all new prices");
            tokio::time::sleep(update_interval).await;
        }
    }

    async fn start_websocket(self) -> Result<()> {
        tracing::info!("[🔮 Oracle] Updating prices via HTTP before starting WebSocket...");
        self.update_prices_http().await?;

        tracing::info!("[🔮 Oracle] Starting WebSocket connection...");
        let ws_url = format!("wss://ws.{}/node/v1/data/subscribe", self.oracle.api_base);

        let connect_result = connect_async(ws_url).await;
        let (ws_stream, _) = match connect_result {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!(
                    "[🔮 Oracle] Failed to establish WebSocket connection: {:?}",
                    e
                );
                return Err(anyhow::anyhow!("WebSocket connection failed"));
            }
        };

        let (mut write, mut read) = ws_stream.split();

        let pairs = {
            let prices = self.latest_prices.0.lock().await;
            let assets: Vec<String> = prices.keys().cloned().collect();
            assets
                .iter()
                .map(|asset| format!("{}/{}", asset, USD_ASSET))
                .collect::<Vec<String>>()
        };

        let subscribe_message = serde_json::json!({
            "msg_type": "subscribe",
            "pairs": pairs
        });

        tracing::info!(
            "[🔮 Oracle] Subscribing to price feeds: {}",
            pairs.join(", ")
        );

        let subscribe_message_str = serde_json::to_string(&subscribe_message)?;

        if let Err(e) = write.send(Message::Text(subscribe_message_str)).await {
            tracing::error!("[🔮 Oracle] Failed to send subscription message: {:?}", e);
            return Err(anyhow::anyhow!("Failed to send subscription message"));
        }

        tracing::info!("[🔮 Oracle] Subscription message sent. Waiting for updates...");
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.handle_ws_message(&text).await {
                        tracing::error!("Error handling WebSocket message: {:#}", e);
                    }
                }
                Ok(_) => {
                    tracing::debug!("[🔮 Oracle] Received non-text message");
                }
                Err(e) => {
                    tracing::error!("WebSocket error: {:?}", e);
                    break;
                }
            }
        }

        tracing::warn!("[🔮 Oracle] WebSocket connection closed unexpectedly");
        Ok(())
    }

    async fn handle_ws_message(&self, message: &str) -> Result<()> {
        let price_data: WebSocketPriceData = serde_json::from_str(message)?;

        let mut prices = self.latest_prices.0.lock().await;

        for oracle_price in price_data.oracle_prices {
            let asset_pair = starknet::core::utils::parse_cairo_short_string(
                &starknet::core::types::Felt::from_hex(&oracle_price.global_asset_id)?,
            )
            .unwrap_or_default();

            let re = Regex::new(r"/?usd$").unwrap();
            let asset = re.replace(&asset_pair.to_lowercase(), "").into_owned();

            if !asset.is_empty() {
                if let Some(asset_info) = prices.get_mut(&asset) {
                    let mut raw_price = oracle_price.median_price;
                    let decimal_places = 18 - asset_info.decimals as usize;

                    // Find existing dot position from the back
                    let current_dot_position = raw_price
                        .rfind('.')
                        .map(|pos| raw_price.len() - pos - 1)
                        .unwrap_or(0);

                    // Remove the current decimal point if it exists
                    raw_price = raw_price.replace(".", "");

                    // Calculate new dot position from the back
                    let new_dot_position = current_dot_position + decimal_places;

                    // Ensure the string is long enough, pad with zeros if necessary
                    while raw_price.len() <= new_dot_position {
                        raw_price.insert(0, '0');
                    }

                    // Apply the dot
                    if new_dot_position < raw_price.len() {
                        raw_price.insert(raw_price.len() - new_dot_position, '.');
                    } else {
                        raw_price.insert(0, '0');
                        raw_price.insert(1, '.');
                    }

                    let price = BigDecimal::from_str(&raw_price).unwrap_or_default();
                    asset_info.price = price.clone();

                    tracing::info!("[🔮 Oracle] Updated price for {}: {}", asset, price);
                } else {
                    tracing::warn!("[🔮 Oracle] Received update for unknown asset: {}", asset);
                }
            } else {
                tracing::warn!("[🔮 Oracle] Invalid asset pair received: {}", asset_pair);
            }
        }

        drop(prices);

        Ok(())
    }

    async fn update_prices_http(&self) -> Result<()> {
        let assets = {
            let prices = self.latest_prices.0.lock().await;
            prices.keys().cloned().collect::<Vec<String>>()
        };

        let fetch_tasks = assets.into_iter().map(|asset| {
            let oracle = self.oracle.clone();
            async move {
                let price_info = oracle.get_dollar_price(asset.clone()).await;
                (asset, price_info)
            }
        });

        let results = join_all(fetch_tasks).await;

        let mut prices = self.latest_prices.0.lock().await;
        for (asset, price_result) in results {
            if let Ok((price, decimals)) = price_result {
                prices.insert(asset, AssetInfo { price, decimals });
            }
        }
        drop(prices);

        Ok(())
    }
}

#[derive(Deserialize, Debug)]
struct WebSocketPriceData {
    oracle_prices: Vec<OraclePrice>,
}

#[derive(Deserialize, Debug)]
struct OraclePrice {
    global_asset_id: String,
    median_price: String,
}

#[derive(Default, Clone)]
pub struct LatestOraclePrices(pub Arc<Mutex<HashMap<String, AssetInfo>>>);

#[derive(Clone, Debug)]
pub struct AssetInfo {
    pub price: BigDecimal,
    pub decimals: i64,
}

impl LatestOraclePrices {
    pub fn from_config(config: &Config) -> Self {
        let mut prices = HashMap::new();
        for asset in config.assets.iter() {
            prices.insert(
                asset.ticker.to_lowercase(),
                AssetInfo {
                    price: BigDecimal::default(),
                    decimals: 0,
                },
            );
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
    api_base: String,
    api_key: String,
    aggregation_method: AggregationMethod,
    interval: Interval,
    network: String,
}

impl PragmaOracle {
    pub fn new(api_base: String, api_key: String, network: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            api_base,
            api_key,
            aggregation_method: AggregationMethod::Median,
            interval: Interval::OneMinute,
            network,
        }
    }

    pub async fn get_dollar_price(&self, asset_name: String) -> Result<(BigDecimal, i64)> {
        let retry_strategy = ExponentialBackoff::from_millis(100).take(3);

        Retry::spawn(retry_strategy, || self.fetch_price(&asset_name))
            .await
            .map_err(|e| anyhow!("Failed to fetch price after retries: {:?}", e))
    }

    async fn fetch_price(&self, asset_name: &str) -> Result<(BigDecimal, i64)> {
        let url = self.fetch_price_url(asset_name.to_string(), USD_ASSET.to_owned());
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
        Ok((asset_price, oracle_response.decimals))
    }

    fn fetch_price_url(&self, base: String, quote: String) -> String {
        format!(
            "https://api.{}/node/v1/onchain/{}/{}?network={}&components=false&variations=false&interval={}&aggregation={}",
            self.api_base, base, quote, self.network, self.interval, self.aggregation_method
        )
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, Display)]
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
