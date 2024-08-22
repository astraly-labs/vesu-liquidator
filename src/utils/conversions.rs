use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::num_bigint::BigInt;
use bigdecimal::BigDecimal;
use starknet::core::types::{Felt, U256};

/// Converts an hexadecimal string with decimals to BigDecimal.
pub fn hex_str_to_big_decimal(hex_price: &str, decimals: i64) -> BigDecimal {
    let cleaned_hex = hex_price.trim_start_matches("0x");
    let price_bigint = BigInt::parse_bytes(cleaned_hex.as_bytes(), 16).unwrap();
    BigDecimal::new(price_bigint, decimals)
}

/// Converts a Felt element from starknet-rs to a FieldElement from Apibara-core.
pub fn felt_as_apibara_field(value: &Felt) -> FieldElement {
    FieldElement::from_bytes(&value.to_bytes_be())
}

/// Converts an Apibara core FieldElement into a Felt from starknet-rs.
pub fn apibara_field_as_felt(value: &FieldElement) -> Felt {
    Felt::from_bytes_be(&value.to_bytes())
}

/// Converts a BigDecimal to a U256.
pub fn big_decimal_to_u256(value: BigDecimal) -> U256 {
    let (amount, _): (BigInt, _) = value.as_bigint_and_exponent();
    U256::from(Felt::from(amount.clone()))
}
