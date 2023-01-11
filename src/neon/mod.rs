pub mod account_storage;
pub mod provider;
pub mod tracer_core;
pub mod neon_cli;

use {
    anyhow::anyhow,
    arrayref::array_ref,
    crate::{
        db::DbClient,
        evm_runtime::EVMRuntime,
        neon::provider::DbProvider,
        v1::{
            geth::types::trace::{ H256T, U256T, H160T },
            types::{BlockNumber, EthCallObject},
        },
    },
    solana_sdk::{ account::Account, account_info::AccountInfo, pubkey::Pubkey },
    std::{
        cell::RefCell, convert::{TryFrom, TryInto}, fmt, rc::Rc, sync::Arc,
    },
    tokio::task::block_in_place,
    web3::{ transports::Http, types::BlockId, Web3 },
    tracing::{ info, warn },
    crate::{
        neon::account_storage::EmulatorAccountStorage,
        syscall_stubs::Stubs,
    },
    log::*,
    serde::Serialize,
    phf::phf_map,

};

pub trait To<T> {
    fn to(self) -> T;
}

type Error = jsonrpsee::types::error::Error;

pub type Result<T> = std::result::Result<T, Error>;




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



#[derive(Debug, Default, Serialize)]
struct EthereumError {
    pub code: u32,
    pub message: Option<String>,
    pub data: Option<String>,
}

const INTERNAL_SERVER_ERROR: fn()->Error = || Error::Custom("Internal server error".to_string());

static ETHEREUM_ERROR_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    "StackUnderflow" => "trying to pop from an empty stack",
    "StackOverflow" => "trying to push into a stack over stack limit",
    "InvalidJump" => "jump destination is invalid",
    "InvalidRange" => "an opcode accesses memory region, but the region is invalid",
    "DesignatedInvalid" => "encountered the designated invalid opcode",
    "CallTooDeep" => "call stack is too deep (runtime)",
    "CreateCollision" => "create opcode encountered collision (runtime)",
    "CreateContractLimit" => "create init code exceeds limit (runtime)",
    "OutOfOffset" => "an opcode accesses external information, but the request is off offset limit (runtime)",
    "OutOfGas" => "execution runs out of gas (runtime)",
    "OutOfFund" => "not enough fund to start the execution (runtime)",
    "PCUnderflow" => "PC underflow (unused)",
    "CreateEmpty" => "attempt to create an empty account (runtime, unused)",
    "StaticModeViolation" => "STATICCALL tried to change state",
};

static ETHEREUM_FATAL_ERROR_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    "NotSupported" => "the operation is not supported",
    "UnhandledInterrupt" => "the trap (interrupt) is unhandled",
    "CallErrorAsFatal" => "the environment explicitly set call errors as fatal error",
};

