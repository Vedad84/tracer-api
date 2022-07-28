pub mod account_storage;
pub mod provider;

use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::sync::Arc;

use anyhow::{anyhow, bail};
use tracing::{debug, info, warn};

use evm::backend::Apply;
use evm::{ExitReason, Transfer, H160, H256, U256};
use evm_loader::instruction::EvmInstruction;
use evm_loader::transaction::UnsignedTransaction;
use evm_loader::{
    executor::Machine,
    executor_state::{ExecutorState, ExecutorSubstate},
};

use evm_loader::account::{EthereumAccount, EthereumContract};

use solana_program::keccak::hash;
use solana_sdk::message::Message as SolanaMessage;

use crate::db::DbClient;
use crate::syscall_stubs::Stubs;

use account_storage::EmulatorAccountStorage;
use provider::{DbProvider, MapProvider, Provider};
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

pub enum EvmAccount<'a> {
    User(EthereumAccount<'a>),
    Contract(EthereumAccount<'a>, EthereumContract<'a>),
}

use tokio::task::block_in_place;
use crate::v1::geth::types::trace::{H160T, H256T, U256T};
use solana_sdk::account_info::AccountInfo;
use arrayref::{array_ref};
use evm_loader::account::{ACCOUNT_SEED_VERSION};
use evm_loader::account_storage::AccountStorage;
use crate::v1::types::BlockNumber;
use web3::{
    transports::Http,
    Web3,
    types::BlockId
};
use web3::futures::FutureExt;

pub trait To<T> {
    fn to(self) -> T;
}

type Error = anyhow::Error;
use thiserror::Error as ThisErr;

#[derive(Debug, ThisErr)]
pub enum TracerError {
    #[error("Web3 error occured: {0:?}")]
    Web3Error(web3::Error),
    #[error("Failed to obtain block number")]
    ErrorObtainBlocknum,
    #[error("Unsupported block tag")]
    UnsupportedBlockTag,
    #[error("Emulator reached step limit")]
    StepLimitReached,
}

impl From<web3::Error> for TracerError {
    fn from(err: web3::Error) -> Self {
        TracerError::Web3Error(err)
    }
}

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

fn convert_h256(inp: H256T) -> web3::types::H256 {
    let bytes = array_ref![inp.0.as_bytes(), 0, 32];
    web3::types::H256::from(bytes)
}

impl TracerCore {
    fn get_block_number(&self, tag: BlockNumber) -> Result<u64, TracerError> {
        match tag {
            BlockNumber::Num(num) => Ok(num),
            BlockNumber::Hash { hash, .. } => {

                info!("Get block number {:?}", hash.0.to_string());

                let future = self.web3
                    .eth()
                    .block(BlockId::Hash(convert_h256(hash)));

                let result = block_in_place(|| {
                    let handle = tokio::runtime::Handle::current();
                    handle.block_on(future)
                })?;


                info!("Web3 part ready");

                Ok(result
                    .ok_or(TracerError::ErrorObtainBlocknum)?
                    .number
                    .ok_or(TracerError::ErrorObtainBlocknum)?
                    .as_u64())
            },
            _ => {
                Err(TracerError::UnsupportedBlockTag)
            }
        }
    }

    pub fn get_storage_at(
        &self,
        contract_id: &H160T,
        index: &U256T,
        tag: BlockNumber,
    ) -> Result<U256T, Error> {

        let block_number = self.get_block_number(tag)?;
        let provider = DbProvider::new(
            self.db_client.clone(),
            self.evm_loader,
        );

        let syscall_stubs = Stubs::new(&provider, Some(block_number))?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

        let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));
        Ok(U256T(account_storage.storage(&contract_id.0, &index.0)))
    }

    pub fn get_balance(
        &self,
        address: &H160T,
        tag: BlockNumber,
    ) -> Result<U256T, Error> {

        let block_number = self.get_block_number(tag)?;
        let provider = DbProvider::new(
            self.db_client.clone(),
            self.evm_loader,
        );

        let syscall_stubs = Stubs::new(&provider, Some(block_number))?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

        let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));
        Ok(U256T(account_storage.balance(&address.0)))
    }

    pub fn eth_call(
        &self,
        from: Option<H160>,
        to: H160,
        gas: Option<U256>,
        gas_price: Option<U256>,
        value: Option<U256>,
        data: Option<Vec<u8>>,
        tag: BlockNumber,
    ) -> Result<String, Error> {

        let block_number = self.get_block_number(tag)?;
        let provider = DbProvider::new(
            self.db_client.clone(),
            self.evm_loader,
        );

        let caller_id = from.unwrap_or_default();
        let block_number = Some(block_number);

        let syscall_stubs = Stubs::new(&provider, block_number)?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

        let storage = EmulatorAccountStorage::new(provider, block_number);
        let mut executor =
            Machine::new(caller_id, &storage)?;

        // u64::MAX is too large, remix gives this error:
        // Gas estimation errored with the following message (see below).
        // Number can only safely store up to 53 bits
        let gas_limit = U256::from(gas.unwrap_or_else(|| 50_000_000u32.into()));

        debug!(
            "call_begin(caller_id={:?}, contract_id={:?}, data={:?}, value={:?})",
            caller_id,
            to,
            data.as_ref().map(|vec| hex::encode(&vec)),
            value,
        );

        executor.call_begin(
            caller_id,
            to,
            data.unwrap_or_default(),
            value.unwrap_or_default(),
            gas_limit,
            gas_price.unwrap_or_default(),
        )?;

        let (result, exit_reason) = match executor.execute_n_steps(100_000) {
            Ok(()) => bail!("bad account kind"),
            Err(result) => result,
        };

        let status = match exit_reason {
            ExitReason::Succeed(_) => "succeed".to_string(),
            ExitReason::Error(_) => "error".to_string(),
            ExitReason::Revert(_) => "revert".to_string(),
            ExitReason::Fatal(_) => "fatal".to_string(),
            ExitReason::StepLimitReached => return Err(Error::from(TracerError::StepLimitReached)),
        };

        if status.eq("succeed") {
            return Ok(format!("0x{}", hex::encode(&result)));
        }

        Ok("0x".to_string())
    }

    pub fn get_code(
        &self,
        address: &H160T,
        tag: BlockNumber,
    ) -> Result<String, Error> {
        let block_number = self.get_block_number(tag)?;
        let provider = DbProvider::new(
            self.db_client.clone(),
            self.evm_loader,
        );

        let syscall_stubs = Stubs::new(&provider, Some(block_number))?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

        let code = EmulatorAccountStorage::new(provider, Some(block_number))
            .code(&address.0);
        Ok(format!("0x{}", hex::encode(code)))
    }

    pub fn get_transaction_count(
        &self,
        account_id: &H160T,
        tag: BlockNumber,
    ) -> Result<U256T, Error> {
        let block_number = self.get_block_number(tag)?;
        let provider = DbProvider::new(
            self.db_client.clone(),
            self.evm_loader,
        );

        let syscall_stubs = Stubs::new(&provider, Some(block_number))?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

        let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));
        Ok(U256T(account_storage.nonce(&account_id.0)))
    }
}

#[must_use]
pub fn keccak256_h256(data: &[u8]) -> H256 {
    H256::from(hash(data).to_bytes())
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
