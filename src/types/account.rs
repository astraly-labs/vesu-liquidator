use std::sync::Arc;

use anyhow::Result;
use bigdecimal::BigDecimal;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::{
        chain_id,
        types::{BlockId, BlockTag, Call, Felt},
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};

pub struct StarknetAccount(
    pub Arc<SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>>,
);

// TODO: Create an Account builder to be able to configure:
// - the chain
// - the method of creation (keystore, raw key...)
impl StarknetAccount {
    /// Create a Starknet account from a provided private_key
    pub fn from_secret(
        rpc_client: Arc<JsonRpcClient<HttpTransport>>,
        account_address: Felt,
        private_key: Felt,
    ) -> StarknetAccount {
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));
        let mut account = SingleOwnerAccount::new(
            rpc_client.clone(),
            signer,
            account_address,
            // TODO: configure chain
            chain_id::MAINNET,
            ExecutionEncoding::New,
        );
        account.set_block_id(BlockId::Tag(BlockTag::Pending));

        StarknetAccount(Arc::new(account))
    }

    /// Returns the account_address of the Account.
    pub async fn account_address(&self) -> Felt {
        self.0.address()
    }

    /// Simulate a set of TXs and return the estimation of the fee necessary
    /// to execute them.
    pub async fn estimate_fees_cost(&self, txs: &[Call]) -> Result<BigDecimal> {
        // We unwrap() the return value to assert that we are not expecting
        // threads to ever fail while holding the lock.
        let account = self.0.clone();
        let estimation = account.execute_v1(txs.to_vec());
        let estimation = estimation.estimate_fee().await?;
        Ok(BigDecimal::new(
            estimation.overall_fee.to_bigint(),
            18,
        ))
    }

    // TODO
    pub async fn execute_txs(&self, _txs: &[Call]) -> Result<()> {
        Ok(())
    }
}
