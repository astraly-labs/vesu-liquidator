use starknet_rust::core::types::U256;

use crate::types::cairo::I129;

// Decimals are always 18 for vesu response
pub const VESU_RESPONSE_DECIMALS: i64 = 18;
pub const MAX_RETRIES_VERIFY_TX_FINALITY: usize = 10;
pub const INTERVAL_CHECK_TX_FINALITY: u64 = 3;

pub const U256_ZERO: U256 = U256::from_words(0, 0);
pub const I129_ZERO: I129 = I129 {
    mag: 0,
    sign: false,
};
