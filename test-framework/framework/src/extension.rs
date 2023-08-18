use neon_cli_lib::types::IndexerDb;
use web3::{transports::Http, Web3};

/// Additional fields used in some test framework methods. See `TestFramework::extended()` for
/// more information.
pub struct TestFrameworkExtension {
    pub web3: Web3<Http>,
    pub faucet_url: String,
    pub indexer: IndexerDb,
}
