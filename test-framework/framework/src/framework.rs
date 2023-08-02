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

use crate::db_types::{AccountInfo, RetrievedTime};

pub struct TestFramework {
    rng: ThreadRng,
    web3: Web3<Http>,
    faucet_url: String,
    clickhouse: clickhouse::Client,
    // TODO: Make indexer optional?
    indexer: IndexerDb,
}

impl TestFramework {
    /// Creates a new test framework instance.
    ///
    /// # Panics
    ///
    /// Panics if the required environment variables aren't set.
    pub async fn new() -> Self {
        let rng = rand::thread_rng();

        let proxy_url =
            env::var("PROXY_URL").expect("Failed to read the PROXY_URL environment variable");
        let web3 = Web3::new(Http::new(&proxy_url).expect("Failed to create Web3 transport"));

        let faucet_url =
            env::var("FAUCET_URL").expect("Failed to read the FAUCET_URL environment variable");

        let db_url = env::var("DB_URL").expect("Failed to read the DB_URL environment variable");
        let clickhouse = clickhouse::Client::default().with_url(&db_url);

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

        Self {
            rng,
            web3,
            faucet_url,
            clickhouse,
            indexer,
        }
    }

    /// Creates a new wallet with the given balance.
    pub async fn make_wallet(&mut self, balance: usize) -> (SecretKey, Address) {
        trace!("Creating a wallet with {balance} balance");

        let secp = Secp256k1::new();
        let (secret_key, _public_key) = secp.generate_keypair(&mut self.rng);
        let address = (&secret_key).address();

        if balance > 0 {
            let wallet = format!("0x{}", hex::encode(address));
            reqwest::Client::new()
                .post(&self.faucet_url)
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

    pub async fn balance(&self, address: Address) -> U256 {
        self.web3
            .eth()
            .balance(address, Some(BlockNumber::Pending))
            .await
            .unwrap_or_else(|_| panic!("Failed to get balance for {}", hex::encode(address)))
    }

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
        self.web3
            .accounts()
            .sign_transaction(params, key)
            .await
            .expect("Failed to sign transaction")
    }

    pub async fn send_raw_transaction(&self, bytes: Bytes) {
        self.web3
            .eth()
            .send_raw_transaction(bytes)
            .await
            .expect("Failed to send raw transaction");
    }

    /// Returns true if an __Ethereum__ transaction with the given hash is present in the
    /// ClickHouse database.
    pub async fn is_known_transaction(&self, hash: H256) -> bool {
        let signature = match self.solana_signature(hash).await {
            Some(s) => s,
            None => return false,
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

            if let Some(s) = self.solana_signature(hash).await {
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

    pub async fn transaction_retrieved_time(&self, hash: H256) -> OffsetDateTime {
        let signature = self
            .solana_signature(hash)
            .await
            .unwrap_or_else(|| panic!("Unable to get solana signature for '{hash:?}' transaction"));
        self.clickhouse
            .query("SELECT retrieved_time FROM events.notify_transaction_distributed WHERE signature = ?")
            .bind(signature.as_slice())
            .fetch_one::<RetrievedTime>()
            .await
            .unwrap_or_else(|_| panic!("Failed to get retrieved_time for '{hash:?}' transaction"))
            .into()
    }

    /// Returns the solana signature from the given Ethereum transaction hash.
    async fn solana_signature(&self, hash: H256) -> Option<[u8; 64]> {
        self.indexer.get_sol_sig(hash.as_fixed_bytes()).await.ok()
    }

    /// Returns a number of accounts in the `events.update_account_distributed` table.
    pub async fn count_accounts(&self) -> usize {
        self.clickhouse
            .query("SELECT COUNT(*) FROM events.update_account_distributed FINAL")
            .fetch_one()
            .await
            .expect("Failed to count update_account_distributed")
    }

    pub async fn account_pubkey(&self, offset: usize) -> Vec<u8> {
        self.clickhouse
            .query("SELECT pubkey FROM events.update_account_distributed FINAL LIMIT 1 OFFSET ?")
            .bind(offset)
            .fetch_one()
            .await
            .expect("Failed to get account pubkey")
    }

    pub async fn account(&self, key: &[u8]) -> AccountInfo {
        self.clickhouse
            .query("SELECT owner, lamports, executable, rent_epoch, data FROM events.update_account_distributed WHERE pubkey = ?")
            .bind(key)
            .fetch_one()
            .await
            .expect("Failed to get account information")
    }
}
