use vesu_liquidator::oracle::{PragmaOracle, OracleApiResponse};
use std::{collections::{HashMap, HashSet}, sync::Arc};

// use alloy::primitives::U256;
use starknet::{core::types::{BlockId, BlockTag, EmittedEvent, EventFilter, Felt, FunctionCall, U256}, providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider}};
use url::Url;
use reqwest;
use json;

pub const PUBLIC_MAINNET_RPC : &str = "https://starknet-mainnet.public.blastapi.io";
pub const VESU_SINGLETON_CONTRACT : &str = "0x2545b2e5d519fc230e9cd781046d3a64e092114f07e44771e0d719d148725ef";
pub const VESU_POSITION_UNSAFE_SELECTOR : &str = "0x00ad73c50509760e79eb76f403619b0622dfe64ad5b307f489299f918afb8f9a";

pub const ETH_ADDRESS : &str = "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7";
pub const WBTC_ADDRESS : &str = "0x03fe2b97c1fd336e750087d68b9b867997fd64a2661ff3ca5a7c771641e8e7ac";
pub const USDC_ADDRESS : &str = "0x053c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8";
pub const USDT_ADDRESS : &str = "0x068f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8";
pub const WSTETH_ADDRESS : &str = "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2";
pub const STRK_ADDRESS : &str = "0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d";

pub const ETH_DECIMAL: u32= 18;
pub const WBTC_DECIMAL: u32= 8;
pub const USDC_DECIMAL: u32= 6;
pub const USDT_DECIMAL: u32= 6;
pub const WSTETH_DECIMAL: u32= 18;
pub const STRK_DECIMAL: u32= 18;


#[derive(Default,Clone, Hash, Eq, PartialEq, Debug)]
pub struct Asset{
    name: String,
    address: Felt,
    decimal: u32,
}

pub struct PriceToDollar{
    price: Felt,
    decimal: u32,
}

impl PriceToDollar {
    fn get_dollar_price(&mut self, mut amount : Felt, decimal: u32) -> U256{
        if self.decimal > decimal {
            amount = amount * Felt::from(10_u128.pow(self.decimal - decimal));
        } else if self.decimal < decimal {
            self.price = self.price * Felt::from(10_u128.pow(decimal - self.decimal));
            self.decimal = decimal;
        }

        (U256::from(amount) * U256::from(self.price)) / U256::from(10_u128.pow(self.decimal))
    }
}

impl Asset {
    fn new(address: Felt) -> Asset{
        Asset {name : get_asset_name_for_address(address), address : address, decimal : get_decimal_for_address(address)}
    }
}

#[derive(Clone, Hash, Eq, PartialEq,Debug)]
pub struct PositionState{
    collateral_amount: Felt,
    debt_amount: Felt,
    decimal: u32,
    ltv_ratio: U256,
}

impl Default for PositionState {
    fn default() -> Self {
        PositionState {
            collateral_amount: Felt::ZERO,
            debt_amount: Felt::ZERO,
            decimal: 0,
            ltv_ratio: U256::from(0u32),
        }
    }
}

impl PositionState {
    fn new(raw_response: Vec<Felt>) -> PositionState{
        PositionState {collateral_amount: raw_response[4],debt_amount: raw_response[6], decimal: 0, ltv_ratio : U256::from(0u32)}
    }

    
}

#[derive(Default,Clone, Hash, Eq, PartialEq,Debug)]
pub struct Position{
    pool_id: Felt,
    collateral_asset: Asset,
    debt_asset: Asset,
    state: PositionState,
}

impl Position {
    fn adapt_decimal(&mut self){
        self.state.decimal = self.collateral_asset.decimal;
        if self.collateral_asset.decimal > self.debt_asset.decimal {
            self.state.debt_amount = self.state.debt_amount * Felt::from(10_u128.pow(self.collateral_asset.decimal - self.debt_asset.decimal));
        } else if self.collateral_asset.decimal < self.debt_asset.decimal {
            self.state.collateral_amount = self.state.collateral_amount * Felt::from(10_u128.pow(self.debt_asset.decimal - self.collateral_asset.decimal));
            self.state.decimal = self.debt_asset.decimal;
        }
    }

    async fn compute_ltv_ratio(&mut self){
        let mut col_price_to_dollar = get_dollar_price_from_pragma_api(self.collateral_asset.name.to_lowercase()).await;
        let col_state_to_dollar = col_price_to_dollar.get_dollar_price(self.state.collateral_amount, self.state.decimal);
        
        let mut brw_price_to_dollar = get_dollar_price_from_pragma_api(self.debt_asset.name.to_lowercase()).await;
        let brw_state_to_dollar = brw_price_to_dollar.get_dollar_price(self.state.debt_amount, self.state.decimal);

        self.state.ltv_ratio = brw_state_to_dollar / col_state_to_dollar;
    }
}

impl Position{
    async fn request_position_state(&mut self, rpc_provider : &Arc<JsonRpcClient<HttpTransport>>, contract_address : Felt, event_key : Vec<Felt>) {
        let position_unsafe_selector_as_felt = Felt::from_hex(VESU_POSITION_UNSAFE_SELECTOR).expect("failed to parse Vesu position unsafe selector");
        
        let pool_id = event_key[1];
        let collateral_asset = event_key[2];
        let debt_asset = event_key[3];
        let user = event_key[4];
        let position_unsafe_calldata = vec![pool_id,collateral_asset,debt_asset,user];
        
        let get_position_request = &FunctionCall {contract_address: contract_address, entry_point_selector : position_unsafe_selector_as_felt, calldata: position_unsafe_calldata};
        let result = rpc_provider.call(get_position_request, BlockId::Tag(BlockTag::Latest)).await.expect("failed to request position state");
        self.state = PositionState::new(result);
    }
}

async fn get_position(rpc_provider : &Arc<JsonRpcClient<HttpTransport>>, filtered_events : Vec<EmittedEvent>, contract_address : Felt) -> HashMap<Felt,HashSet<Position>>{
    let mut position_per_users: HashMap<Felt,HashSet<Position>> = HashMap::new();
    for event in filtered_events {
        let user_address = event.keys[4];
        let collateral_asset= Asset::new(event.keys[2]);
        let debt_asset= Asset::new(event.keys[3]);

        let mut position = Position {pool_id: event.keys[1],collateral_asset,debt_asset, state: PositionState::default()};
        position.request_position_state(&rpc_provider, contract_address, event.keys).await;
        position.adapt_decimal();
        position.compute_ltv_ratio().await;
        position_per_users.entry(user_address)
                    .or_insert_with(HashSet::new)
                    .insert(position);

    }

    position_per_users
}

async fn retrieve_events(rpc_provider : &Arc<JsonRpcClient<HttpTransport>>, vesu_singleton_address_as_felt : Felt) -> Vec<EmittedEvent>{
    let key_filter: Option<Vec<Vec<Felt>>> = Some(vec![vec![Felt::from_hex("0x3dfe6670b0f4e60f951b8a326e7467613b2470d81881ba2deb540262824f1e").unwrap()]]);
    let filter: EventFilter = EventFilter {from_block: Some(BlockId::Number(668220)), to_block : Some(BlockId::Tag(BlockTag::Latest)), address: Some(vesu_singleton_address_as_felt), keys: key_filter};
    let events = rpc_provider.get_events(filter, None, 100).await.expect("failed to retrieve events");
    
    let filtered_events = events.events.into_iter().filter(|event| {
        event.keys.get(3).map_or(false, |&key| key != Felt::ZERO)
    }).collect::<Vec<_>>();

    return filtered_events;
}

fn get_asset_name_for_address(address: Felt) -> String{
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

fn get_decimal_for_address(address: Felt) -> u32{
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

async fn get_dollar_price_from_pragma_api(asset_name : String) -> PriceToDollar {
    let pragma_oracle = PragmaOracle::default();
    let response = reqwest::Client::new()
        .get(pragma_oracle.get_fetch_url(String::from(asset_name), String::from("usd")))
        .header("x-api-key", pragma_oracle.get_api_key())
        .send()
        .await.expect("failed to retrieve price from pragma api");
    let hr_resp = response.json::<OracleApiResponse>().await.expect("failed to serialize api result into struct");
    PriceToDollar {price : Felt::from_hex(hr_resp.price.as_str()).unwrap(), decimal : hr_resp.decimals}
}

#[tokio::main]
async fn main() {
    let rpc_url : Url = Url::parse(PUBLIC_MAINNET_RPC).expect("failed to parse RPC URL");
    let rpc_provider : Arc<JsonRpcClient<HttpTransport>> = Arc::new(JsonRpcClient::new(HttpTransport::new(rpc_url)));
    let vesu_singleton_address_as_felt = Felt::from_hex(VESU_SINGLETON_CONTRACT).expect("failed to parse vesu address as felt");

    let events = retrieve_events(&rpc_provider, vesu_singleton_address_as_felt).await;
    
    let position_per_users: HashMap<Felt,HashSet<Position>> = get_position(&rpc_provider, events, vesu_singleton_address_as_felt).await;

    println!("users {:#?}", position_per_users);

}
