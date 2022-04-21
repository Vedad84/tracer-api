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

//use solana_client::rpc_client::RpcClient;
use solana_program::keccak::hash;
use solana_sdk::message::Message as SolanaMessage;

use crate::db::DbClient as RpcClient;
use crate::syscall_stubs::Stubs;

use account_storage::EmulatorAccountStorage;
use provider::{DbProvider, MapProvider, Provider};
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

pub enum EvmAccount<'a> {
    User(EthereumAccount<'a>),
    Contract(EthereumAccount<'a>, EthereumContract<'a>),
}

use solana_sdk::account_info::AccountInfo;
use arrayref::{array_ref};
use evm_loader::account::{ACCOUNT_SEED_VERSION};
use evm_loader::account_storage::AccountStorage;

pub trait To<T> {
    fn to(self) -> T;
}

type Error = anyhow::Error;


#[derive(Clone)]
pub struct Config {
    pub rpc_client: Arc<RpcClient>,
    //pub websocket_url: String,
    pub evm_loader: Pubkey,
    // #[allow(unused)]
    // fee_payer: Pubkey,
    //signer: Box<dyn Signer + Send>,
    //pub keypair: Option<Keypair>,
    pub rpc_client_after: Arc<RpcClient>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            //"evm_loader={:?}, signer={:?}",
            "evm_loader={:?}",
            self.evm_loader //, self.signer
        )
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

pub fn get_storage_at<P>(
    provider: P,
    contract_id: &H160,
    index: &U256,
    block_number: u64)
    -> U256
where  P: Provider, {
    let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));
    account_storage.storage(contract_id, index)
}

pub fn get_balance<P>(
    provider: P,
    address: &H160,
    block_number: u64)
    -> U256
    where  P: Provider, {
    let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));
    account_storage.balance(address)
}

pub fn eth_call<P: Provider> (
    provider: P,
    from: Option<H160>,
    to: H160,
    gas: Option<U256>,
    value: Option<U256>,
    data: Option<Vec<u8>>,
    block_number: u64,
) -> Result<String, Error> {
    let caller_id = from.unwrap_or_default();
    let block_number = Some(block_number);

    let syscall_stubs = Stubs::new(&provider, block_number)?;
    solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

    let storage = EmulatorAccountStorage::new(provider, block_number);
    let mut executor = Machine::new(caller_id, &storage)?;

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
        ExitReason::StepLimitReached => unreachable!(),
    };

    if status.eq("succeed") {
        return Ok(format!("0x{}", hex::encode(&result)));
    }

    Ok("0x".to_string())
}

pub fn get_code<P: Provider>(
    provider: P,
    address: &H160,
    block_number: u64,
) -> Vec<u8> {
    EmulatorAccountStorage::new(provider, Some(block_number))
        .code(address)
}

pub fn get_transaction_count<P>(
    provider: P,
    account_id: &H160,
    block_number: u64,
) -> U256
    where  P: Provider, {
    let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));
    account_storage.nonce(account_id)
}
