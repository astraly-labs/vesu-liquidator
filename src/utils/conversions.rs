use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::BigDecimal;
use bigdecimal::num_bigint::BigInt;
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
    U256::from(big_decimal_to_felt(value))
}

pub fn big_decimal_to_felt(value: BigDecimal) -> Felt {
    let (amount, _): (BigInt, _) = value.as_bigint_and_exponent();
    Felt::from(amount.clone())
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use bigdecimal::{BigDecimal, num_bigint::BigInt};

    use crate::utils::conversions::hex_str_to_big_decimal;

    #[test]
    fn test_hex_str_to_decimal() {
        assert_eq!(
            hex_str_to_big_decimal("0x100000000000", 3),
            BigDecimal::new(BigInt::from_str("17592186044416").unwrap(), 3)
        );
        assert_eq!(
            hex_str_to_big_decimal("100000000000", 3),
            BigDecimal::new(BigInt::from_str("17592186044416").unwrap(), 3)
        );
    }
}
