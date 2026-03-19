use std::sync::Arc;

use starknet_rust::providers::{JsonRpcClient, jsonrpc::HttpTransport};

pub mod account;
pub mod asset;
pub mod position;

pub type StarknetSingleOwnerAccount = Arc<
    starknet_rust::accounts::SingleOwnerAccount<
        Arc<JsonRpcClient<HttpTransport>>,
        starknet_rust::signers::LocalWallet,
    >,
>;
