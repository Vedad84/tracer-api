use std::collections::HashMap;
use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

use tracing::warn;

use evm::backend::Apply;
use evm::{H160, U256};
use evm_loader::{
    account_data::{AccountData, ACCOUNT_SEED_VERSION},
    executor_state::{ERC20Approve, SplApprove, SplTransfer},
    solana_backend::{AccountStorage, AccountStorageInfo},
    solidity_account::SolidityAccount,
};

use solana_program::instruction::AccountMeta;
use solana_sdk::{account::Account, pubkey::Pubkey};

use super::provider::Provider;
use crate::neon::Config;
use crate::utils::parse_token_amount;

pub fn make_solana_program_address(ether_address: &H160, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&[ACCOUNT_SEED_VERSION], ether_address.as_bytes()],
        program_id,
    )
}

struct SolanaAccount {
    account: Account,
    code_account: Option<Account>,
    key: Pubkey,
    writable: bool,
    code_size: Option<usize>,
    balance: u64,
}

impl SolanaAccount {
    pub fn new(account: Account, key: Pubkey, balance: u64, code_account: Option<Account>) -> Self {
        eprintln!("SolanaAccount::new");
        Self {
            account,
            key,
            balance,
            writable: false,
            code_account,
            code_size: None,
        }
    }
}

macro_rules! bail_with_default {
    ($opt:expr, $fun:expr) => {
        match $opt {
            Some(value) => value,
            None => return $fun(),
        }
    };
}

#[allow(clippy::module_name_repetitions)]
pub struct EmulatorAccountStorage<P> {
    accounts: RefCell<HashMap<H160, SolanaAccount>>,
    provider: P,
    contract_id: H160,
    caller_id: H160,
    block_number: u64,
    block_timestamp: i64,
}

impl<P: Provider> EmulatorAccountStorage<P> {
    pub fn new(
        provider: P,
        contract_id: H160,
        caller_id: H160,
        block_number: Option<u64>,
    ) -> EmulatorAccountStorage<P> {
        eprintln!("backend::new");

        let slot = block_number.unwrap_or_else(|| {
            if let Ok(slot) = provider.get_slot() {
                eprintln!("Got slot");
                eprintln!("Slot {}", slot);
                slot
            } else {
                eprintln!("Get slot error");
                0
            }
        });

        let timestamp = if let Ok(timestamp) = provider.get_block_time(slot) {
            eprintln!("Got timestamp");
            eprintln!("timestamp {}", timestamp);
            timestamp
        } else {
            eprintln!("Get timestamp error");
            0
        };

        Self {
            accounts: RefCell::new(HashMap::new()),
            provider,
            contract_id,
            caller_id,
            block_number: slot,
            block_timestamp: timestamp,
        }
    }

    pub fn fetch_account(&self, pubkey: &Pubkey, slot: u64) -> Option<Account> {
        self.provider.get_account_at_slot(pubkey, slot).ok()? // TODO: warning
    }

    fn init_neon_account(&self, address: H160) {
        let mut accounts = self.accounts.borrow_mut();

        macro_rules! return_none {
            ($opt:expr) => {
                bail_with_default!($opt, || ())
            };
        }

        if !accounts.contains_key(&address) {
            let (solana_address, _nonce) =
                make_solana_program_address(&address, &self.provider.evm_loader());

            // Note: CLI logic will add the address to new_accounts map.
            // Note: In our case we always work with created accounts.
            let solana_account =
                return_none!(self.fetch_account(&solana_address, self.block_number));

            let account_data = return_none!(AccountData::unpack(&solana_account.data)
                .ok()
                .and_then(|data| match data {
                    AccountData::Account(data) => Some(data),
                    _ => None,
                }));

            let code = (account_data.code_account != Pubkey::new_from_array([0_u8; 32]))
                .then(|| {
                    let code = self.fetch_account(&account_data.code_account, self.block_number);
                    if code.is_none() {
                        warn!(
                            neon_account_key = %solana_address,
                            code_account_key = %account_data.code_account,
                            "code account not found"
                        );
                    }
                    code
                })
                .flatten();

            let balance;
            let amount = self
                .fetch_account(&account_data.eth_token_account, self.block_number)
                .map(|account| parse_token_amount(&account)?.amount.parse().ok());

            match amount {
                Some(Some(amount)) => balance = amount,
                Some(None) => {
                    warn!(
                        neon_account_key = %solana_address,
                        token_account_key = %account_data.eth_token_account,
                        "could not parse token account"
                    );
                    balance = 0;
                }
                None => {
                    warn!(
                        neon_account_key = %solana_address,
                        token_account_key = %account_data.eth_token_account,
                        "token account not found"
                    );
                    balance = 0;
                }
            }

            let account = SolanaAccount::new(solana_account.clone(), solana_address, balance, code);
            accounts.insert(address, account);
        }
    }
}

impl<P: Provider> AccountStorage for EmulatorAccountStorage<P> {
    fn apply_to_account<U, D, F>(&self, address: &H160, d: D, f: F) -> U
    where
        F: FnOnce(&SolidityAccount<'_>) -> U,
        D: FnOnce() -> U,
    {
        macro_rules! ward {
            ($opt:expr) => {
                bail_with_default!($opt, d)
            };
        }
        self.init_neon_account(*address);
        let mut accounts = self.accounts.borrow_mut();

        let account = ward!(accounts.get(address));
        let account_data = ward!(AccountData::unpack(&account.account.data)
            .ok()
            .filter(|data| matches!(data, AccountData::Account(_))));

        let mut code_data;
        let mut code = None;
        if let Some(ref code_account) = account.code_account {
            code_data = code_account.data.clone();
            let contract_data = ward!(AccountData::unpack(&code_account.data)
                .ok()
                .filter(|data| matches!(data, AccountData::Contract(_))));
            let code_data = Rc::new(RefCell::new(code_data.as_mut()));
            code = Some((contract_data, code_data));
        }

        let account = SolidityAccount::new(&account.key, account_data, code);
        f(&account)
    }

    fn apply_to_solana_account<U, D, F>(&self, address: &Pubkey, d: D, f: F) -> U
    where
        F: FnOnce(&AccountStorageInfo) -> U,
        D: FnOnce() -> U,
    {
        let mut account = bail_with_default!(self.fetch_account(address, self.block_number), d);
        f(&account_storage_info(&mut account))
    }

    fn balance(&self, address: &H160) -> U256 {
        self.init_neon_account(*address);
        self.accounts
            .borrow()
            .get(address)
            .map_or(U256::zero(), |acc| acc.balance.into())
            * evm_loader::token::eth::min_transfer_value() // Why??
    }

    fn program_id(&self) -> &Pubkey {
        &self.provider.evm_loader()
    }

    fn contract(&self) -> H160 {
        self.contract_id
    }

    fn origin(&self) -> H160 {
        self.caller_id
    }

    fn block_number(&self) -> U256 {
        self.block_number.into()
    }

    fn block_timestamp(&self) -> U256 {
        self.block_timestamp.into()
    }

    fn get_account_solana_address(&self, address: &H160) -> Pubkey {
        make_solana_program_address(address, &self.provider.evm_loader()).0
    }
}

/// Creates new instance of `AccountStorageInfo` from `Account`.
fn account_storage_info(account: &mut Account) -> AccountStorageInfo {
    AccountStorageInfo {
        lamports: account.lamports,
        data: Rc::new(RefCell::new(&mut account.data)),
        owner: &account.owner,
        executable: account.executable,
        rent_epoch: account.rent_epoch,
    }
}
