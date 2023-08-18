//! Provides utilities for testing.

use std::{
    env,
    time::{Duration, Instant},
};

use neon_cli_lib::types::{ChDbConfig, IndexerDb};
use rand::rngs::ThreadRng;
use secp256k1::Secp256k1;
use serde_json::json;
use time::OffsetDateTime;
use tracing::trace;
use web3::{
    signing::{Key, SecretKey},
    transports::Http,
    types::{
        Address, BlockNumber, Bytes, CallRequest, SignedTransaction, TransactionParameters, H256,
        U256,
    },
    Web3,
};

use crate::{
    db_types::{AccountInfo, RetrievedTime},
    extension::TestFrameworkExtension,
};

pub struct TestFramework<E> {
    rng: ThreadRng,
    clickhouse: clickhouse::Client,
    extension: E,
}

impl TestFramework<()> {
    /// Creates a new test framework instance suitable for basic operations.
    ///
    /// # Panics
    ///
    /// Panics if the `DB_URL` environment variables aren't set.
    pub fn basic() -> Self {
        Self::create(())
    }

    /// Creates a new test framework instance with connection to the Indexer database and web3
    /// functionality.
    ///
    /// # Panics
    ///
    /// Panics if the required environment variables aren't set.
    pub async fn extended() -> TestFramework<TestFrameworkExtension> {
        let proxy_url =
            env::var("PROXY_URL").expect("Failed to read the PROXY_URL environment variable");
        let web3 = Web3::new(Http::new(&proxy_url).expect("Failed to create Web3 transport"));

        let faucet_url =
            env::var("FAUCET_URL").expect("Failed to read the FAUCET_URL environment variable");

        let indexer_host = env::var("DB_INDEXER_HOST")
            .expect("Failed to read DB_INDEXER_HOST environment variable");
        let indexer_port = env::var("DB_INDEXER_PORT")
            .expect("Failed to read DB_INDEXER_PORT environment variable");
        let indexer_database = env::var("DB_INDEXER_DATABASE")
            .expect("Failed to read DB_INDEXER_DATABASE environment variable");
        let indexer_user = env::var("DB_INDEXER_USER")
            .expect("Failed to read DB_INDEXER_USER environment variable");
        let indexer_password = env::var("INDEXER_DB_PASSWORD")
            .expect("Failed to read INDEXER_DB_PASSWORD environment variable");
        let indexer_config = ChDbConfig {
            clickhouse_url: Vec::new(),
            clickhouse_user: None,
            clickhouse_password: None,
            indexer_host,
            indexer_port,
            indexer_database,
            indexer_user,
            indexer_password,
        };
        let indexer = IndexerDb::new(&indexer_config).await;

        TestFramework::create(TestFrameworkExtension {
            web3,
            faucet_url,
            indexer,
        })
    }

    /// Creates a new test framework instance __with__ connection to the Indexer database.
    ///
    /// # Panics
    ///
    /// Panics if the required environment variables aren't set.
    pub async fn with_indexer() -> TestFramework<IndexerDb> {
        let indexer_host = env::var("DB_INDEXER_HOST")
            .expect("Failed to read DB_INDEXER_HOST environment variable");
        let indexer_port = env::var("DB_INDEXER_PORT")
            .expect("Failed to read DB_INDEXER_PORT environment variable");
        let indexer_database = env::var("DB_INDEXER_DATABASE")
            .expect("Failed to read DB_INDEXER_DATABASE environment variable");
        let indexer_user = env::var("DB_INDEXER_USER")
            .expect("Failed to read DB_INDEXER_USER environment variable");
        let indexer_password = env::var("INDEXER_DB_PASSWORD")
            .expect("Failed to read INDEXER_DB_PASSWORD environment variable");
        let indexer_config = ChDbConfig {
            clickhouse_url: Vec::new(),
            clickhouse_user: None,
            clickhouse_password: None,
            indexer_host,
            indexer_port,
            indexer_database,
            indexer_user,
            indexer_password,
        };
        let indexer = IndexerDb::new(&indexer_config).await;

        TestFramework::create(indexer)
    }
}

impl<E> TestFramework<E> {
    fn create(extension: E) -> Self {
        let rng = rand::thread_rng();

        let db_url = env::var("DB_URL").expect("Failed to read the DB_URL environment variable");
        let clickhouse = clickhouse::Client::default().with_url(db_url);

        TestFramework {
            rng,
            clickhouse,
            extension,
        }
    }

    /// Returns a number of accounts in the `events.update_account_distributed` table.
    pub async fn count_accounts(&self) -> usize {
        self.clickhouse
            .query("SELECT COUNT(*) FROM events.update_account_distributed FINAL")
            .fetch_one()
            .await
            .expect("Failed to count update_account_distributed")
    }

    /// Reads a public key with the given offset from the database.
    pub async fn account_pubkey(&self, offset: usize) -> Vec<u8> {
        self.clickhouse
            .query("SELECT pubkey FROM events.update_account_distributed LIMIT 1 OFFSET ?")
            .bind(offset)
            .fetch_one()
            .await
            .expect("Failed to get account pubkey")
    }

    /// Returns an account information with the given key.
    pub async fn account(&self, key: &[u8]) -> AccountInfo {
        self.clickhouse
            .query("SELECT owner, lamports, executable, rent_epoch, data FROM events.update_account_distributed WHERE pubkey = ?")
            .bind(key)
            .fetch_one()
            .await
            .expect("Failed to get account information")
    }
}

impl TestFramework<TestFrameworkExtension> {
    /// Creates a new wallet with the given balance.
    pub async fn make_wallet(&mut self, balance: usize) -> (SecretKey, Address) {
        trace!("Creating a wallet with {balance} balance");

        let secp = Secp256k1::new();
        let (secret_key, _public_key) = secp.generate_keypair(&mut self.rng);
        let address = (&secret_key).address();

        if balance > 0 {
            let wallet = format!("0x{}", hex::encode(address));
            reqwest::Client::new()
                .post(&self.extension.faucet_url)
                .json(&json!({
                    "wallet": wallet,
                    "amount": balance,
                }))
                .send()
                .await
                .expect("Failed to post deposit request");
        }

        (secret_key, address)
    }

    /// Returns a balance of the given wallet.
    pub async fn balance(&self, address: Address) -> U256 {
        self.extension
            .web3
            .eth()
            .balance(address, Some(BlockNumber::Pending))
            .await
            .unwrap_or_else(|_| panic!("Failed to get balance for {}", hex::encode(address)))
    }

    /// Creates a signed __Ethereum__ transfer transaction.
    pub async fn make_transfer_transaction(
        &self,
        from: Address,
        to: Address,
        value: U256,
        key: &SecretKey,
    ) -> SignedTransaction {
        if U256::from(0) == self.balance(from).await {
            panic!("Wallet {} has zero balance", hex::encode(from));
        }

        let request = CallRequest {
            from: Some(from),
            to: Some(to),
            value: Some(value),
            ..Default::default()
        };
        let gas = self
            .extension
            .web3
            .eth()
            .estimate_gas(request, None)
            .await
            .expect("Failed to estimate gas");
        let params = TransactionParameters {
            gas,
            to: Some(to),
            value,
            ..Default::default()
        };
        self.extension
            .web3
            .accounts()
            .sign_transaction(params, key)
            .await
            .expect("Failed to sign transaction")
    }

    /// Sends the given bytes as a transaction.
    pub async fn send_raw_transaction(&self, bytes: Bytes) {
        self.extension
            .web3
            .eth()
            .send_raw_transaction(bytes)
            .await
            .expect("Failed to send raw transaction");
    }

    /// Returns true if an __Ethereum__ transaction with the given hash is present in the
    /// ClickHouse database.
    pub async fn is_known_transaction(&self, hash: H256) -> bool {
        let signature = match self
            .extension
            .indexer
            .get_sol_sig(hash.as_fixed_bytes())
            .await
        {
            Ok(s) => s,
            Err(_) => return false,
        };
        1 == self
            .clickhouse
            .query("SELECT COUNT(1) FROM events.notify_transaction_distributed WHERE signature = ?")
            .bind(signature.as_slice())
            .fetch_one::<usize>()
            .await
            .expect("Failed to check transaction")
    }

    /// Waits for the transaction to appear in the ClickHouse database. Returns false if isn't
    /// found even after the timeout. Performs a busy wait loop if no sleep value is specified.
    pub async fn wait_for_transaction(
        &self,
        hash: H256,
        timeout: Duration,
        sleep: Option<Duration>,
    ) -> bool {
        let mut now = Instant::now();
        let timeout = now + timeout;

        let signature = loop {
            now = Instant::now();

            if now > timeout {
                return false;
            }

            if let Ok(s) = self
                .extension
                .indexer
                .get_sol_sig(hash.as_fixed_bytes())
                .await
            {
                break s;
            } else if let Some(t) = sleep {
                tokio::time::sleep(t).await;
            }
        };

        while now < timeout {
            now = Instant::now();

            if 1 == self
                .clickhouse
                .query("SELECT COUNT(1) FROM events.notify_transaction_distributed WHERE signature = ?")
                .bind(signature.as_slice())
                .fetch_one::<usize>()
                .await
                .expect("Failed to check transaction") {
                return true;
            } else if let Some(t) = sleep {
                tokio::time::sleep(t).await;
            }
        }

        false
    }

    /// Same as `wait_for_transaction`, but with default values for sleep (200 milliseconds) and
    /// timeout (10 seconds).
    pub async fn wait_for_transaction_default(&self, hash: H256) -> bool {
        self.wait_for_transaction(
            hash,
            Duration::from_secs(10),
            Some(Duration::from_millis(200)),
        )
        .await
    }

    /// Returns the retrieved time of the transaction.
    pub async fn transaction_retrieved_time(&self, hash: H256) -> OffsetDateTime {
        let signature = self
            .extension
            .indexer
            .get_sol_sig(hash.as_fixed_bytes())
            .await
            .unwrap_or_else(|e| {
                panic!("Unable to get solana signature for '{hash:?}' transaction: {e:?}")
            });
        self.clickhouse
            .query("SELECT retrieved_time FROM events.notify_transaction_distributed WHERE signature = ?")
            .bind(signature.as_slice())
            .fetch_one::<RetrievedTime>()
            .await
            .unwrap_or_else(|_| panic!("Failed to get retrieved_time for '{hash:?}' transaction"))
            .into()
    }
}
