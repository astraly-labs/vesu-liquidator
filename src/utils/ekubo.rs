use anyhow::{Context, Result};
use bigdecimal::BigDecimal;
use cainome::cairo_serde::{ContractAddress, U256};
use serde_json::Value;
use starknet::core::types::Felt;

use crate::bindings::liquidate::{PoolKey, RouteNode};

const EKUBO_QUOTE_ENDPOINT: &str = "https://mainnet-api.ekubo.org/quote";
const QUOTE_QUERY_PARAMS: &str = "maxHops=0&maxSplits=0";

const MAX_SQRT_RATIO: U256 = U256 {
    low: 147820330697885451836970967903133202728,
    high: 18446739710271796309,
};

pub async fn get_ekubo_route(
    amount: BigDecimal,
    decimal: i64,
    from_token: String,
    to_token: String,
) -> Result<Vec<RouteNode>> {
    let amount_as_string = format!("-{}e{}", amount, decimal);
    let ekubo_api_endpoint = format!(
        "{EKUBO_QUOTE_ENDPOINT}/{amount_as_string}/{from_token}/{to_token}?{QUOTE_QUERY_PARAMS}"
    );
    tracing::info!("{}", ekubo_api_endpoint);
    let http_client = reqwest::Client::new();

    let response = http_client.get(ekubo_api_endpoint).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("API request failed with status: {}", response.status());
    }

    let response_text = response.text().await?;
    let json_value: Value = serde_json::from_str(&response_text)?;

    let splits = json_value["splits"]
        .as_array()
        .context("'splits' is not an array")?;

    let mut routes = Vec::new();
    for split in splits {
        // Process routes for this split
        let split_routes = split["route"]
            .as_array()
            .context("'route' is not an array")?
            .iter()
            .map(|node| {
                let pool_key = &node["pool_key"];
                Ok(RouteNode {
                    pool_key: PoolKey {
                        token0: ContractAddress(Felt::from_hex(
                            pool_key["token0"]
                                .as_str()
                                .context("token0 is not a string")?,
                        )?),
                        token1: ContractAddress(Felt::from_hex(
                            pool_key["token1"]
                                .as_str()
                                .context("token1 is not a string")?,
                        )?),
                        fee: u128::from_str_radix(
                            pool_key["fee"]
                                .as_str()
                                .context("fee is not a string")?
                                .trim_start_matches("0x"),
                            16,
                        )
                        .context("Failed to parse fee as u128")?,
                        tick_spacing: pool_key["tick_spacing"]
                            .as_u64()
                            .context("tick_spacing is not a u64")?
                            as u128,
                        extension: ContractAddress(Felt::from_hex(
                            pool_key["extension"]
                                .as_str()
                                .context("extension is not a string")?,
                        )?),
                    },
                    sqrt_ratio_limit: MAX_SQRT_RATIO,
                    skip_ahead: node["skip_ahead"]
                        .as_u64()
                        .context("skip_ahead is not a u64")?
                        as u128,
                })
            })
            .collect::<Result<Vec<RouteNode>>>()?;

        routes.extend(split_routes);
    }
    Ok(routes)
}
