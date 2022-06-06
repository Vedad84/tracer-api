use clickhouse::Client;
use thiserror::Error;

use evm::{H160, H256};
use solana_account_decoder::parse_token::{
    parse_token, TokenAccountType, UiTokenAccount, UiTokenAmount,
};
use solana_sdk::account::{Account, ReadableAccount};
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use tokio::task::block_in_place;
use tracing::debug;

use crate::utils::parse_token_amount;

type Slot = u64;

pub struct DbClient {
    client: Client,
    use_acc_after_trx: bool,
}

impl std::fmt::Debug for DbClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DbClient{{}}")
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("clickhouse: {}", .0)]
    Db(#[from] clickhouse::error::Error),
}

#[derive(Debug, serde::Deserialize, clickhouse::Row, Clone)]
struct AccountRow {
    pubkey: [u8; 32],
    lamports: u64,
    data: Vec<u8>,
    owner: [u8; 32],
    executable: bool,
    rent_epoch: u64,
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Account {
        Account {
            lamports: row.lamports,
            data: row.data,
            owner: Pubkey::new_from_array(row.owner),
            executable: row.executable,
            rent_epoch: row.rent_epoch,
        }
    }
}

type DbResult<T> = std::result::Result<T, Error>;

impl DbClient {
    pub fn new(
        addr: impl Into<String>,
        user: Option<String>,
        password: Option<String>,
        db: Option<String>,
        use_acc_after_trx: bool
    ) -> Self {
        let client = Client::default().with_url(addr);
        let client = if let Some(user) = user {
            client.with_user(user)
        } else {
            client
        };
        let client = if let Some(password) = password {
            client.with_password(password)
        } else {
            client
        };
        let client = if let Some(db) = db {
            client.with_database(db)
        } else {
            client
        };
        DbClient { client, use_acc_after_trx }
    }


    fn block<F, Fu, R>(&self, f: F) -> R
    where
        F: FnOnce(Client) -> Fu,
        Fu: std::future::Future<Output = R>,
    {
        let client = self.client.clone();
        block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(f(client))
        })
    }

    #[tracing::instrument]
    pub fn get_slot(&self) -> Result<Slot, Error> {
        let slot = self.block(|client| async move {
            client
                .query("SELECT max(slot) FROM transactions")
                .fetch_one::<u64>()
                .await
        })?;
        Ok(slot)
    }

    #[tracing::instrument]
    pub fn get_block_time(&self, slot: Slot) -> Result<i64, Error> {
        let time = self.block(|client| async move {
            client
                .query("SELECT toUnixTimestamp(date_time) from transactions where slot = ?")
                .bind(slot)
                .fetch_one::<i64>()
                .await
        })?;
        Ok(time)
    }

    fn get_accounts_table(&self) -> &str {
        static ACCOUNTS_AFTER_TABLE: &str = "accounts_after_transaction";
        static ACCOUNTS_TABLE: &str = "accounts";

        if self.use_acc_after_trx {
            ACCOUNTS_AFTER_TABLE
        } else {
            ACCOUNTS_TABLE
        }
    }

    pub fn get_accounts_at_slot(
        &self,
        pubkeys: impl Iterator<Item = Pubkey>,
        slot: Slot,
    ) -> DbResult<Vec<(Pubkey, Account)>> {
        let pubkeys = pubkeys
            .map(|pubkey| hex::encode(&pubkey.to_bytes()[..]))
            .fold(String::new(), |old, addr| {
                format!("{} unhex('{}'),", old, addr)
            });

        let accounts = self.block(|client| async move {
            client
                .query(&format!(
                    "SELECT
                        public_key,
                        argMax(lamports, T.slot),
                        argMax(data, T.slot),
                        argMax(owner,T.slot),
                        argMax(executable,T.slot),
                        argMax(rent_epoch,T.slot)
                     FROM {} A
                     JOIN transactions T
                     ON A.transaction_signature = T.transaction_signature
                     WHERE T.slot <= ? AND public_key IN ({})
                     GROUP BY public_key",
                    self.get_accounts_table(), pubkeys
                ))
                .bind(slot)
                .fetch_all::<AccountRow>()
                .await
        })?;
        let accounts = accounts
            .into_iter()
            .map(|row| (Pubkey::new_from_array(row.pubkey), Account::from(row)))
            .collect();
        debug!("found account: {:?}", accounts);
        Ok(accounts)
    }

    #[tracing::instrument]
    pub fn get_account_at_slot(&self, pubkey: &Pubkey, slot: Slot) -> DbResult<Option<Account>> {
        let accounts = self.get_accounts_at_slot(std::iter::once(pubkey.to_owned()), slot)?;
        let account = accounts.get(0).map(|(_, account)| account).cloned();
        Ok(account)
    }
}
