use anyhow::Result;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::utils::conversions::hexa_price_to_big_decimal;

// TODO: API URL should be a CLI arg
pub const DEV_API_URL: &str = "https://api.dev.pragma.build/node/v1/data/";

pub const USD_ASSET: &str = "usd";

#[derive(Deserialize, Debug)]
pub struct OracleApiResponse {
    pub price: String,
    pub decimals: i64,
}

#[derive(Debug, Clone)]
pub struct PragmaOracle {
    http_client: reqwest::Client,
    pub api_url: String,
    pub api_key: String,
    pub aggregation_method: AggregationMethod,
    pub interval: Interval,
}

impl PragmaOracle {
    pub fn new(api_key: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            api_url: DEV_API_URL.to_owned(),
            api_key,
            aggregation_method: AggregationMethod::Median,
            // TODO: Assert that we want OneMinute
            interval: Interval::OneMinute,
        }
    }
}

impl PragmaOracle {
    pub fn fetch_price_url(&self, base: String, quote: String) -> String {
        format!(
            "{}{}/{}?interval={}&aggregation={}",
            self.api_url, base, quote, self.interval, self.aggregation_method
        )
    }

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
