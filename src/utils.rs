use lazy_static::lazy_static;
use starknet::core::types::Felt;

use bigdecimal::BigDecimal;

pub const PUBLIC_MAINNET_RPC: &str = "https://starknet-mainnet.public.blastapi.io";
pub const VESU_SINGLETON_CONTRACT_ADDRESS: &str =
    "0x2545b2e5d519fc230e9cd781046d3a64e092114f07e44771e0d719d148725ef";
pub const VESU_POSITION_UNSAFE_SELECTOR_ADDRESS: &str =
    "0x00ad73c50509760e79eb76f403619b0622dfe64ad5b307f489299f918afb8f9a";
pub const EXTENSION_CONTRACT_ADDRESS: &str =
    "0x2334189e831d804d4a11d3f71d4a982ec82614ac12ed2e9ca2f8da4e6374fa";
pub const MODIFY_POSITION_SELECTOR: &str =
    "0x3dfe6670b0f4e60f951b8a326e7467613b2470d81881ba2deb540262824f1e";

pub const ETH_ADDRESS: &str = "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7";
pub const WBTC_ADDRESS: &str = "0x03fe2b97c1fd336e750087d68b9b867997fd64a2661ff3ca5a7c771641e8e7ac";
pub const USDC_ADDRESS: &str = "0x053c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8";
pub const USDT_ADDRESS: &str = "0x068f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8";
pub const WSTETH_ADDRESS: &str =
    "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2";
pub const STRK_ADDRESS: &str = "0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d";

pub const ETH_DECIMAL: u32 = 18;
pub const WBTC_DECIMAL: u32 = 8;
pub const USDC_DECIMAL: u32 = 6;
pub const USDT_DECIMAL: u32 = 6;
pub const WSTETH_DECIMAL: u32 = 18;
pub const STRK_DECIMAL: u32 = 18;

lazy_static! {
    pub static ref EXTENSION_CONTRACT: Felt = Felt::from_hex(EXTENSION_CONTRACT_ADDRESS).unwrap();
    pub static ref MODIFY_POSITION_EVENT: Felt = Felt::from_hex(MODIFY_POSITION_SELECTOR).unwrap();
    pub static ref VESU_SINGLETON_CONTRACT: Felt =
        Felt::from_hex(VESU_SINGLETON_CONTRACT_ADDRESS).unwrap();
    pub static ref VESU_POSITION_UNSAFE_SELECTOR: Felt =
        Felt::from_hex(VESU_POSITION_UNSAFE_SELECTOR_ADDRESS)
            .expect("failed to parse Vesu position unsafe selector");
}

pub fn get_asset_name_for_address(address: Felt) -> String {
    match address {
        address if address == Felt::from_hex(ETH_ADDRESS).unwrap() => String::from("ETH"),
        address if address == Felt::from_hex(WBTC_ADDRESS).unwrap() => String::from("WBTC"),
        address if address == Felt::from_hex(USDC_ADDRESS).unwrap() => String::from("USDC"),
        address if address == Felt::from_hex(USDT_ADDRESS).unwrap() => String::from("USDT"),
        address if address == Felt::from_hex(WSTETH_ADDRESS).unwrap() => String::from("WSTETH"),
        address if address == Felt::from_hex(STRK_ADDRESS).unwrap() => String::from("STRK"),
        _ => String::default(),
    }
}

pub fn get_decimal_for_address(address: Felt) -> u32 {
    match address {
        address if address == Felt::from_hex(ETH_ADDRESS).unwrap() => ETH_DECIMAL,
        address if address == Felt::from_hex(WBTC_ADDRESS).unwrap() => WBTC_DECIMAL,
        address if address == Felt::from_hex(USDC_ADDRESS).unwrap() => USDC_DECIMAL,
        address if address == Felt::from_hex(USDT_ADDRESS).unwrap() => USDT_DECIMAL,
        address if address == Felt::from_hex(WSTETH_ADDRESS).unwrap() => WSTETH_DECIMAL,
        address if address == Felt::from_hex(STRK_ADDRESS).unwrap() => STRK_DECIMAL,
        _ => 0,
    }
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
