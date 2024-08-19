use bigdecimal::num_bigint::BigInt;
use bigdecimal::BigDecimal;

pub fn hexa_price_to_big_decimal(hex_price: &str, decimals: u32) -> BigDecimal {
    let cleaned_hex = hex_price.trim_start_matches("0x");
    let price_bigint = BigInt::parse_bytes(cleaned_hex.as_bytes(), 16).unwrap();
    BigDecimal::new(price_bigint, decimals as i64)
}

pub fn normalize_to_decimals(
    value: BigDecimal,
    original_decimals: u32,
    target_decimals: u32,
) -> BigDecimal {
    if target_decimals >= original_decimals {
        let power = BigDecimal::from(10_i64.pow(target_decimals - original_decimals));
        value * power
    } else {
        let power = BigDecimal::from(10_i64.pow(original_decimals - target_decimals));
        value / power
    }
}
