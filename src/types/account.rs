use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use bigdecimal::BigDecimal;
use starknet::{
    accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount},
    core::{
        chain_id,
        types::{BlockId, BlockTag, Felt},
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};

use crate::{
    cli::{NetworkName, RunCmd},
    utils::constants::VESU_RESPONSE_DECIMALS,
};

pub struct StarknetAccount(
    pub Arc<SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>>,
);

impl StarknetAccount {
    /// Creates a StarknetAccount from the CLI args
    pub fn from_cli(
        rpc_client: Arc<JsonRpcClient<HttpTransport>>,
        run_cmd: RunCmd,
    ) -> Result<StarknetAccount> {
        let mut builder = StarknetAccountBuilder::default();

        builder = match run_cmd.network {
            NetworkName::Mainnet => builder.on_mainnet(),
            NetworkName::Sepolia => builder.on_sepolia(),
            NetworkName::Devnet => builder.on_mainnet(),
        };

        builder = builder
            .as_account(run_cmd.account_params.account_address)
            .with_provider(rpc_client);

        if let Some(private_key) = run_cmd.account_params.private_key {
            builder.from_secret(private_key)
        } else {
            builder.from_keystore(
                run_cmd.account_params.keystore_path.unwrap(),
                &run_cmd.account_params.keystore_password.unwrap(),
            )
        }
    }

    /// Returns the account_address of the Account.
    pub fn account_address(&self) -> Felt {
        self.0.address()
    }

    /// Simulate a set of TXs and return the estimation of the fee necessary
    /// to execute them.
    pub async fn estimate_fees_cost(&self, txs: &[Call]) -> Result<BigDecimal> {
        let estimation = self.0.execute_v1(txs.to_vec()).estimate_fee().await?;
        Ok(BigDecimal::new(
            estimation.overall_fee.to_bigint(),
            VESU_RESPONSE_DECIMALS,
        ))
    }

    /// Executes a set of transactions and returns the transaction hash.
    pub async fn execute_txs(&self, txs: &[Call]) -> Result<Felt> {
        let res = self.0.execute_v1(txs.to_vec()).send().await?;
        Ok(res.transaction_hash)
    }
}

#[derive(Debug, Default)]
pub struct StarknetAccountBuilder {
    account_address: Option<Felt>,
    chain_id: Option<Felt>,
    rpc_client: Option<Arc<JsonRpcClient<HttpTransport>>>,
}

impl StarknetAccountBuilder {
    pub fn new() -> Self {
        StarknetAccountBuilder::default()
    }

    pub fn on_mainnet(mut self) -> Self {
        self.chain_id = Some(chain_id::MAINNET);
        self
    }

    pub fn on_sepolia(mut self) -> Self {
        self.chain_id = Some(chain_id::SEPOLIA);
        self
    }
    pub fn as_account(mut self, account_address: Felt) -> Self {
        self.account_address = Some(account_address);
        self
    }

    pub fn with_provider(mut self, rpc_client: Arc<JsonRpcClient<HttpTransport>>) -> Self {
        self.rpc_client = Some(rpc_client);
        self
    }

    pub fn from_secret(self, private_key: Felt) -> Result<StarknetAccount> {
        let signing_key = SigningKey::from_secret_scalar(private_key);
        let signer = LocalWallet::from(signing_key);
        self.build(signer)
    }

    pub fn from_keystore(
        self,
        keystore_path: PathBuf,
        keystore_password: &str,
    ) -> Result<StarknetAccount> {
        let signing_key = SigningKey::from_keystore(keystore_path, keystore_password)?;
        let signer = LocalWallet::from(signing_key);
        self.build(signer)
    }

    fn build(self, signer: LocalWallet) -> Result<StarknetAccount> {
        let mut account = SingleOwnerAccount::new(
            self.rpc_client.unwrap(),
            signer,
            self.account_address.unwrap(),
            self.chain_id.unwrap(),
            ExecutionEncoding::New,
        );

        account.set_block_id(BlockId::Tag(BlockTag::Pending));

        Ok(StarknetAccount(Arc::new(account)))
    }
}
