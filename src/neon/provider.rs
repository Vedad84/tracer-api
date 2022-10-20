use std::{borrow::Borrow, collections::HashMap, convert::Infallible, sync::Arc};

use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use web3::signing::Key;
use crate::geth::{H160T, H256T, U256T};
use crate::v1::types::BlockNumber;
use crate::db::{DbClient, Error as DbError};
use crate::v1::types::{FilterObject, LogObject, FilterAddress};

pub trait Provider {
    type Error: std::fmt::Display + std::error::Error + Send + Sync + 'static;

    fn get_account_at_slot(
        &self,
        pubkey: &Pubkey,
        slot: u64,
    ) -> Result<Option<Account>, Self::Error>;

    fn get_slot(&self) -> Result<u64, Self::Error>;
    fn get_block_time(&self, slot: u64) -> Result<i64, Self::Error>; // TODO: Clock sysvar
    fn evm_loader(&self) -> &Pubkey;
}

pub struct DbProvider {
    client: Arc<DbClient>,
    evm_loader: Pubkey,
}

impl DbProvider {
    pub fn new(client: Arc<DbClient>, evm_loader: Pubkey) -> Self {
        Self { client, evm_loader }
    }

    pub fn get_logs(
        &self,
        block_hash: Option<H256T>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        topics: Option<Vec<H256T>>,
        address: Option<FilterAddress>,
    ) -> Result<Vec<LogObject>, DbError> {
        self.client.get_logs(block_hash, from_block, to_block, topics, address)
    }
}

impl Provider for DbProvider {
    type Error = DbError;

    fn get_account_at_slot(
        &self,
        pubkey: &Pubkey,
        slot: u64,
    ) -> Result<Option<Account>, Self::Error> {
        self.client.get_account_at_slot(pubkey, slot)
    }

    fn get_slot(&self) -> Result<u64, Self::Error> {
        self.client.get_slot()
    }

    fn get_block_time(&self, slot: u64) -> Result<i64, Self::Error> {
        self.client.get_block_time(slot)
    }

    fn evm_loader(&self) -> &Pubkey {
        &self.evm_loader
    }
}
