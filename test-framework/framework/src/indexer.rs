use async_trait::async_trait;
use neon_cli_lib::types::IndexerDb;
use web3::types::H256;

/// This trait is only needed to make some test framework methods to be available conditionally
/// depending on if the indexer database is available or not.
#[async_trait]
pub trait Indexer {
    async fn solana_signature(&self, hash: H256) -> Option<[u8; 64]>;
}

#[async_trait]
impl Indexer for IndexerDb {
    async fn solana_signature(&self, hash: H256) -> Option<[u8; 64]> {
        self.get_sol_sig(hash.as_fixed_bytes()).await.ok()
    }
}
