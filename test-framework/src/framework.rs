//! Provides utilities for testing.

use std::{env, time::Duration};

use neon_cli_lib::types::{ChDbConfig, IndexerDb, PgError};
use rand::rngs::ThreadRng;
use secp256k1::Secp256k1;
use serde_json::json;
use time::OffsetDateTime;
use web3::{
    signing::{Key, SecretKey},
    transports::Http,
    types::{
        Address, BlockNumber, Bytes, CallRequest, SignedTransaction, TransactionParameters, H256,
        U256,
    },
    Web3,
};

use crate::db_types::RetrievedTime;

pub struct TestFramework {
    rng: ThreadRng,
    web3: Web3<Http>,
    faucet_url: String,
    clickhouse: clickhouse::Client,
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
    pub async fn make_wallet(&mut self, balance: u32) -> (SecretKey, Address) {
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
            .expect(&format!(
                "Failed to get balance for {}",
                hex::encode(address)
            ))
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

    pub async fn update_account_max_retrieved_time(&self) -> OffsetDateTime {
        self.clickhouse
            .query("SELECT max(retrieved_time) FROM events.update_account_distributed FINAL")
            .fetch_one::<RetrievedTime>()
            .await
            .expect("Failed to get max retrieved_time for accounts")
            .into()
    }

    /// Returns true if an __Ethereum__ transaction with the given hash is present in the
    /// ClickHouse database.
    pub async fn is_known_transaction(&self, hash: H256) -> bool {
        let signature = match self.indexer.get_sol_sig(hash.as_fixed_bytes()).await {
            Ok(s) => s,
            // It would be better to check the error kind (RowCount::RowCount), but it is
            // unfortunately private.
            Err(PgError::Db(_)) => return false,
            Err(e) => panic!("Indexer::get_sol_sig failed: {e:?}"),
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
    /// found even after the timeout.
    pub async fn wait_for_transaction(&self, hash: H256) -> bool {
        for _ in 0..50 {
            if self.is_known_transaction(hash).await {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        false
    }
}
