use anyhow::{Context, Result};
use bigdecimal::BigDecimal;
use cainome::cairo_serde::{ContractAddress, U256};
use serde_json::Value;
use starknet::core::types::Felt;

use crate::{
    bindings::liquidate::{PoolKey, RouteNode, Swap, TokenAmount},
    utils::constants::I129_ZERO,
};

const EKUBO_QUOTE_ENDPOINT: &str = "https://quoter-mainnet-api.ekubo.org";
const SCALE: u128 = 1_000_000_000_000_000_000;

pub async fn get_ekubo_route(
    http_client: &reqwest::Client,
    from_token: Felt,
    to_token: Felt,
    amount: &BigDecimal,
) -> Result<(Vec<Swap>, Vec<u128>)> {
    let (scaled_amount, _) = amount.as_bigint_and_exponent();

    let ekubo_api_endpoint = format!(
        "{}/-{}/{}/{}",
        EKUBO_QUOTE_ENDPOINT,
        scaled_amount,
        from_token.to_fixed_hex_string(),
        to_token.to_fixed_hex_string()
    );

    let response = http_client.get(ekubo_api_endpoint).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("API request failed with status: {}", response.status());
    }

    let response_text = response.text().await?;
    let json_value: Value = serde_json::from_str(&response_text)?;

    let splits = json_value["splits"]
        .as_array()
        .context("'splits' is not an array")?;

    if splits.is_empty() {
        anyhow::bail!("No splits returned from Ekubo API");
    }

    // Handle single split case (100% weight)
    if splits.len() == 1 {
        let route = parse_route(&splits[0])?;
        return Ok((
            vec![Swap {
                route,
                token_amount: TokenAmount {
                    token: ContractAddress(from_token),
                    amount: I129_ZERO,
                },
            }],
            vec![SCALE], // Single weight of 100%
        ));
    }

    // Calculate total amount for weight calculation
    let total_amount: i128 = splits
        .iter()
        .map(|split| {
            split["amount_specified"]
                .as_str()
                .unwrap_or("0")
                .parse::<i128>()
                .unwrap_or(0)
        })
        .sum();

    let mut swaps = Vec::with_capacity(splits.len());
    let mut weights = Vec::with_capacity(splits.len());
    let mut running_weight_sum: u128 = 0;

    // Process all splits except the last one
    for split in splits.iter().take(splits.len() - 1) {
        let split_amount = split["amount_specified"]
            .as_str()
            .context("amount_specified is not a string")?
            .parse::<i128>()?;

        let weight = (split_amount.unsigned_abs() * SCALE) / (total_amount.unsigned_abs());
        running_weight_sum += weight;
        weights.push(weight);

        let route = parse_route(split)?;
        swaps.push(Swap {
            route,
            token_amount: TokenAmount {
                token: ContractAddress(from_token),
                amount: I129_ZERO,
            },
        });
    }

    // Handle the last split - ensure exact SCALE total
    let last_split = splits.last().unwrap();
    let last_weight = SCALE - running_weight_sum;
    weights.push(last_weight);

    let route = parse_route(last_split)?;
    swaps.push(Swap {
        route,
        token_amount: TokenAmount {
            token: ContractAddress(from_token),
            amount: I129_ZERO,
        },
    });

    // Verify total is exactly SCALE
    let total_weight: u128 = weights.iter().sum();
    assert!(total_weight == SCALE, "Weights do not sum to SCALE");

    Ok((swaps, weights))
}

fn parse_route(split: &Value) -> Result<Vec<RouteNode>> {
    split["route"]
        .as_array()
        .context("'route' is not an array")?
        .iter()
        .map(|node| {
            let pool_key = &node["pool_key"];
            let sqrt_ratio_limit = node["sqrt_ratio_limit"]
                .as_str()
                .context("sqrt_ratio_limit is not a string")?;

            let sqrt_ratio = U256::from_bytes_be(&Felt::from_hex(sqrt_ratio_limit)?.to_bytes_be());

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
                sqrt_ratio_limit: sqrt_ratio,
                skip_ahead: node["skip_ahead"]
                    .as_u64()
                    .context("skip_ahead is not a u64")? as u128,
            })
        })
        .collect()
}
