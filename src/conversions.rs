use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::num_bigint::BigInt;
use bigdecimal::BigDecimal;
use starknet::core::types::Felt;

pub fn hexa_price_to_big_decimal(hex_price: &str, decimals: i64) -> BigDecimal {
    let cleaned_hex = hex_price.trim_start_matches("0x");
    let price_bigint = BigInt::parse_bytes(cleaned_hex.as_bytes(), 16).unwrap();
    BigDecimal::new(price_bigint, decimals)
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

/// Converts a Felt element from starknet-rs to a FieldElement from Apibara-core.
pub fn felt_as_apibara_field_element(value: &Felt) -> FieldElement {
    FieldElement::from_bytes(&value.to_bytes_be())
}

/// Converts an Apibara core FieldElement into a Felt from starknet-rs.
pub fn apibara_field_element_as_felt(value: &FieldElement) -> Felt {
    Felt::from_bytes_be(&value.to_bytes())
}
