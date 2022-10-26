pub mod account_storage;
pub mod provider;

use {
    anyhow::anyhow,
    arrayref::array_ref,
    crate::{
        db::DbClient,
        neon::provider::DbProvider,
        v1::{
            geth::types::trace::{ H256T },
            types::BlockNumber,
        },
    },
    solana_sdk::{ account::Account, account_info::AccountInfo, pubkey::Pubkey },
    std::{
        cell::RefCell, convert::{TryFrom, TryInto}, fmt, rc::Rc, sync::Arc,
    },
    tokio::task::block_in_place,
    web3::{ transports::Http, types::BlockId, Web3 },
    tracing::{ info, warn },
};

pub trait To<T> {
    fn to(self) -> T;
}

type Error = jsonrpsee::types::error::Error;

#[derive(Clone)]
pub struct TracerCore {
    evm_loader: Pubkey,
    tracer_db_client: Arc<DbClient>,
    indexer_db_client: Arc<DbClient>,
    web3: Arc<Web3<Http>>,
}

pub type Result<T> = std::result::Result<T, Error>;

fn convert_h256(inp: H256T) -> web3::types::H256 {
    let bytes = array_ref![inp.0.as_bytes(), 0, 32];
    web3::types::H256::from(bytes)
}

impl TracerCore {
    pub fn new(
        evm_loader: Pubkey,
        tracer_db_client: Arc<DbClient>,
        indexer_db_client: Arc<DbClient>,
        web3: Arc<Web3<Http>>,
    ) -> Self {
        Self {
            evm_loader,
            tracer_db_client,
            indexer_db_client,
            web3,
        }
    }

    pub fn tracer_db_provider(&self) -> DbProvider {
        DbProvider::new(self.tracer_db_client.clone(), self.evm_loader)
    }

    pub fn indexer_db_provider(&self) -> DbProvider {
        DbProvider::new(self.indexer_db_client.clone(), self.evm_loader)
    }

    pub fn get_block_number(&self, tag: BlockNumber) -> Result<u64> {
        match tag {
            BlockNumber::Num(num) => Ok(num),
            BlockNumber::Hash { hash, .. } => {

                let hash_str = hash.0.to_string();
                info!("Get block number {:?}", &hash_str);

                let future = self.web3
                    .eth()
                    .block(BlockId::Hash(convert_h256(hash)));

                let result = block_in_place(|| {
                    let handle = tokio::runtime::Handle::current();
                    handle.block_on(future)
                }).map_err(|err| Error::Custom(format!("Failed to get block number: {:?}", err)))?;

                info!("Web3 part ready");

                Ok(result
                    .ok_or(Error::Custom(format!("Failed to obtain block number for hash: {}", hash_str)))?
                    .number
                    .ok_or(Error::Custom(format!("Failed to obtain block number for hash: {}", hash_str)))?
                    .as_u64())
            },
            BlockNumber::Earliest => {
                self.tracer_db_client.get_earliest_slot().map_err(
                    |err| Error::Custom(format!("Failed to retrieve earliest block: {:?}", err))
                )
            },
            BlockNumber::Latest => {
                self.tracer_db_client.get_slot().map_err(
                    |err| Error::Custom(format!("Failed to retrieve latest block: {:?}", err))
                )
            },
            _ => {
                Err(Error::Custom(format!("Unsupported block tag")))
            }
        }
    }
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
