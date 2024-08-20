pub mod account;

use account::AccountParams;
use url::Url;

fn parse_url(s: &str) -> Result<Url, url::ParseError> {
    s.parse()
}

#[derive(Clone, Debug, clap::Parser)]
pub struct RunCmd {
    #[allow(missing_docs)]
    #[clap(flatten)]
    pub account_params: AccountParams,
    /// The network chain configuration.
    #[clap(long, short, value_name = "NETWORK NAME")]
    pub network: NetworkName,
    /// The rpc endpoint url.
    #[clap(long, value_parser = parse_url, value_name = "RPC URL")]
    pub rpc_url: Url,
    /// The block you want to start syncing from.
    #[clap(long, short, value_name = "BLOCK NUMBER")]
    pub starting_block: u64,
    /// Apibara API Key for indexing.
    #[clap(long, value_name = "APIBARA API KEY")]
    pub apibara_api_key: Option<String>,
    /// Pragma API Key for indexing.
    #[clap(long, value_parser = parse_url, value_name = "PRAGMA API BASE URL")]
    pub pragma_api_base_url: Url,
    /// Pragma API Key for indexing.
    #[clap(long, value_name = "PRAGMA API KEY")]
    pub pragma_api_key: Option<String>,
}

/// Starknet network name.
#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq)]
pub enum NetworkName {
    #[value(alias("mainnet"))]
    Mainnet,
    #[value(alias("sepolia"))]
    Sepolia,
}

impl NetworkName {
    pub fn default_rpc(&self) -> &'static str {
        match self {
            NetworkName::Mainnet => "https://free-rpc.nethermind.io/mainnet-juno",
            NetworkName::Sepolia => "https://free-rpc.nethermind.io/sepolia-juno",
        }
    }
}
