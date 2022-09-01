pub mod account_storage;
pub mod provider;

use {
    anyhow::anyhow,
    crate::db::DbClient,
    solana_sdk::{ account::Account, account_info::AccountInfo, pubkey::Pubkey },
    std::{
        cell::RefCell, convert::{TryFrom, TryInto}, fmt, rc::Rc, sync::Arc,
    },
    web3::{ transports::Http, Web3 },
};

pub trait To<T> {
    fn to(self) -> T;
}

type Error = anyhow::Error;

#[derive(Clone)]
pub struct TracerCore {
    pub evm_loader: Pubkey,
    pub db_client: Arc<DbClient>,
    pub web3: Arc<Web3<Http>>,
}

impl std::fmt::Debug for TracerCore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            //"evm_loader={:?}, signer={:?}",
            "evm_loader={:?}",
            self.evm_loader //, self.signer
        )
    }
}

/// Creates new instance of `AccountInfo` from `Account`.
pub fn account_info<'a>(key: &'a Pubkey, account: &'a mut Account) -> AccountInfo<'a> {
    AccountInfo {
        key,
        is_signer: false,
        is_writable: false,
        lamports: Rc::new(RefCell::new(&mut account.lamports)),
        data: Rc::new(RefCell::new(&mut account.data)),
        owner: &account.owner,
        executable: account.executable,
        rent_epoch: account.rent_epoch,
    }
}
