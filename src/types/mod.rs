use std::sync::Arc;

use starknet::providers::{JsonRpcClient, jsonrpc::HttpTransport};

pub mod account;
pub mod asset;
pub mod position;

pub type StarknetSingleOwnerAccount = Arc<
    starknet::accounts::SingleOwnerAccount<
        Arc<JsonRpcClient<HttpTransport>>,
        starknet::signers::LocalWallet,
    >,
>;
