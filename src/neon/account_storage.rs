use std::convert::TryInto;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

use tracing::warn;

use evm::backend::Apply;
use evm::{H160, H256, U256};
use evm_loader::{
    account_storage::{AccountStorage},
    account::{ACCOUNT_SEED_VERSION, EthereumAccount, EthereumContract, ERC20Allowance, token},
    executor_state::{ERC20Approve, SplApprove, SplTransfer},
    config::STORAGE_ENTIRIES_IN_CONTRACT_ACCOUNT,
    account::EthereumStorage,
};

use evm_loader::account::tag;

use solana_program::sysvar::recent_blockhashes;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    sysvar::{
        clock::Clock,
        Sysvar,
    },
};

use super::provider::Provider;
use solana_sdk::account_info::AccountInfo;
use std::collections::{BTreeMap, BTreeSet};
use solana_sdk::program_error::ProgramError;
use crate::neon::{Error, account_info};
use std::env;
use std::str::FromStr;


macro_rules! bail_with_default {
    ($opt:expr, $fun:expr) => {
        match $opt {
            Some(value) => value,
            None => return $fun(),
        }
    };
}

struct SolanaAccount {
    account: Account,
    code_account: Option<Account>,
    key: Pubkey,
}


#[allow(clippy::module_name_repetitions)]
pub struct EmulatorAccountStorage<P> {
    ethereum_accounts: RefCell<HashMap<H160, SolanaAccount>>,
    solana_accounts: RefCell<HashMap<Pubkey, Account>>,
    provider: P,
    block_number: u64,
    block_timestamp: i64,
    token_mint: Pubkey,
    chain_id: u64,
    clock: Clock,
}

impl<'a, P: Provider> EmulatorAccountStorage<P> {
    pub fn new(
        provider: P,
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

        let token_mint = Pubkey::from_str(
            env::var("NEON_TOKEN_MINT")
                .expect("NEON_TOKEN_MINT is not set").as_str())
            .expect("Unable to parse NEON_TOKEN_MINT");

        let chain_id = u64::from_str(
            env::var("NEON_CHAIN_ID")
                .expect("NEON_CHAIN_ID is not set").as_str())
            .expect("Unable to parse NEON_CHAIN_ID");

        let clock = Clock::get().expect("Failed to create clock");

        Self {
            // accounts: RefCell::new(HashMap::new()),
            ethereum_accounts:  RefCell::new(HashMap::new()),
            solana_accounts:  RefCell::new(HashMap::new()),
            provider: provider,
            block_number: slot,
            block_timestamp: timestamp,
            token_mint,
            chain_id,
            clock,
        }
    }


    fn create_acc_if_not_exists(&self, address: &H160) ->bool{
        // Note: CLI logic will add the address to new_accounts map.
        // Note: In our case we always work with created accounts.

        let mut ether_accounts = self.ethereum_accounts.borrow_mut();

        if !ether_accounts.contains_key(address) {

            let (key, _) = Pubkey::find_program_address(&[&[ACCOUNT_SEED_VERSION], address.as_bytes()],  self.provider.evm_loader());
            let solana = match self.provider.get_account_at_slot(&key, self.block_number){
                Ok(acc) => acc,
                Err(_) => {
                    warn!("error to get_account_at_slot: {}", key);
                    return false
                }
            };

            if solana.is_none(){
                warn!("account not found: {}", key);
                return false
            }
            let mut solana = solana.unwrap();

            let code_key_opt = {
                let info = account_info(&key, &mut solana);

                let ether_account = match EthereumAccount::from_account(self.provider.evm_loader(), &info){
                    Ok(acc) => acc,
                    Err(e) => {
                        warn!("EthereumAccount::from_account() error: {}", key);
                        return false;
                    }
                };
                ether_account.code_account
            };

            let code_account = if let Some(code_key) = code_key_opt {
                let acc = match self.provider.get_account_at_slot(&code_key, self.block_number){
                    Ok(a) => a,
                    Err(_) => {
                        warn!("error to get_account_at_slot: {}", code_key);
                        return false
                    }
                };

                if acc.is_none(){
                    warn!("account not found: {}", code_key);
                    return false
                }
                acc
            }
            else{
                None
            };

            ether_accounts.insert(*address, SolanaAccount{account: solana, code_account: code_account, key: key});
            return true
        }
        true
    }


    fn create_sol_acc_if_not_exists(&self, key: &Pubkey) ->bool{
        let mut solana_accounts = self.solana_accounts.borrow_mut();

        if !solana_accounts.contains_key(key) {
            let acc = self.provider.get_account_at_slot(key, self.block_number).unwrap_or(None);
            if let Some(account) = acc {
                solana_accounts.insert(*key, account);
                return true;
            }
            else {
                return false;
            }
        }

        true
    }


    fn ethereum_account_map_or<F, D>(&self, address: &H160, default: D, f: F) -> D
        where
            F: FnOnce(&EthereumAccount) -> D
    {
        self.create_acc_if_not_exists(address);

        let mut accounts = self.ethereum_accounts.borrow_mut();

        if let Some( solana) = accounts.get_mut(address) {
            let info = account_info(&solana.key, &mut solana.account);

            let ethereum_account = EthereumAccount::from_account(self.provider.evm_loader(), &info).unwrap();
            f(&ethereum_account)
        } else {
            default
        }
    }

    fn ethereum_contract_map_or<F, D>(&self, address: &H160, default: D, f: F) -> D
       where
            F: FnOnce(&EthereumContract) -> D
    {
        if !self.create_acc_if_not_exists(address) {
            warn!("Failed to create/find ethereum account {:?}", address);
            return default
        }

        let mut accounts = self.ethereum_accounts.borrow_mut();
        if let Some(account) = accounts.get_mut(address) {
            const ERR_ETH_NO_CODE_ACC: u32 = 1;
            const ERR_SOL_NO_CODE_ACC: u32 = 2;

            let info = account_info(&account.key, &mut account.account);
            EthereumAccount::from_account(&self.provider.evm_loader(), &info)
                .and_then(|ethereum_account| {
                    ethereum_account.code_account.ok_or(ProgramError::Custom(ERR_ETH_NO_CODE_ACC))
                })
                .and_then(|code_key| {
                    match account.code_account {
                        Some(ref mut code_account) => Ok((code_key, code_account)),
                        None => Err(ProgramError::Custom(ERR_SOL_NO_CODE_ACC))
                    }
                })
                .and_then(|(code_key, code_account)| {
                    let code_info = account_info(&code_key, code_account);
                    let ethereum_contract = EthereumContract::from_account(&self.provider.evm_loader(), &code_info)?;
                    Ok(f(&ethereum_contract))
                })
                .map_or_else(
                    |err| {
                        let err_description = match err {
                            ProgramError::Custom(ERR_ETH_NO_CODE_ACC) => "Ethereum account has no code account".to_string(),
                            ProgramError::Custom(ERR_SOL_NO_CODE_ACC) => "Solana account has no code account".to_string(),
                            err => format!("{:?}", err),
                        };
                        warn!("Failed to get ethereum contract account: {}", err_description);
                        default
                    },
                    |result| result)
        } else {
            default
        }
    }
}

impl<P: Provider> AccountStorage for EmulatorAccountStorage<P> {

    fn program_id(&self) -> &Pubkey {
        self.provider.evm_loader()
    }

    fn balance(&self, address: &H160) -> U256 {
        self.ethereum_account_map_or(address, U256::zero(), |a| a.balance)
    }

    fn block_number(&self) -> U256 {
        self.block_number.into()
    }

    fn block_timestamp(&self) -> U256 {
        self.block_timestamp.into()
    }


    fn nonce(&self, address: &H160) -> U256 {
        self.ethereum_account_map_or(address, 0_u64, |a| a.trx_count).into()
    }

    fn code(&self, address: &H160) -> Vec<u8> {
        self.ethereum_contract_map_or(address,
                                      Vec::new(),
                                      |c| c.extension.code.to_vec()
        )
    }

    fn code_hash(&self, address: &H160) -> H256 {
        self.ethereum_contract_map_or(address,
                                      H256::default(),
                                      |c| evm_loader::utils::keccak256_h256(&c.extension.code)
        )
    }

    fn code_size(&self, address: &H160) -> usize {
        self.ethereum_contract_map_or(address, 0_u32, |c| c.code_size)
            .try_into()
            .expect("usize is 8 bytes")
    }

    fn exists(&self, address: &H160) -> bool {

        self.create_acc_if_not_exists(address);

        let accounts = self.ethereum_accounts.borrow();
        accounts.contains_key(address)
    }


    fn get_spl_token_balance(&self, token_account: &Pubkey) -> u64 {

        self.create_sol_acc_if_not_exists(token_account);

        let mut solana_accounts = self.solana_accounts.borrow_mut();

        if let Some(account) = solana_accounts.get_mut(token_account) {

            let info = account_info(&token_account, account);
            token::State::from_account(&info).map_or(0_u64, |a| a.amount)
        }
        else{
            0_u64
        }
    }

    fn get_spl_token_supply(&self, token_mint: &Pubkey) -> u64 {
        self.create_sol_acc_if_not_exists(token_mint);

        let mut solana_accounts = self.solana_accounts.borrow_mut();

        if let Some(account) = solana_accounts.get_mut(token_mint) {
            let info = account_info(&token_mint, account);
            token::Mint::from_account(&info).map_or(0_u64, |a| a.supply)
        }
        else{
            0_u64
        }
    }

    fn get_spl_token_decimals(&self, token_mint: &Pubkey) -> u8 {
        self.create_sol_acc_if_not_exists(token_mint);

        let mut solana_accounts = self.solana_accounts.borrow_mut();

        if let Some(account) = solana_accounts.get_mut(token_mint) {
            let info = account_info(&token_mint, account);
            token::Mint::from_account(&info).map_or(0_u8, |a| a.decimals)
        }
        else{
            0_u8
        }
    }


    fn get_erc20_allowance(&self, owner: &H160, spender: &H160, contract: &H160, mint: &Pubkey) -> U256 {
        let (sol, _) = self.get_erc20_allowance_address(owner, spender, contract, mint);
        self.create_sol_acc_if_not_exists(&sol);

        let mut solana_accounts = self.solana_accounts.borrow_mut();

        if let Some(account) = solana_accounts.get_mut(&sol) {
            let info = account_info(&sol, account);
            ERC20Allowance::from_account(self.provider.evm_loader(), &info)
                .map_or_else(|_| U256::zero(), |a| a.value)
        }
        else{
            U256::zero()
        }
    }

    fn query_account(&self, key: &Pubkey, data_offset: usize, data_len: usize) -> Option<evm_loader::query::Value> {
        self.create_sol_acc_if_not_exists(key);

        let mut solana_accounts = self.solana_accounts.borrow_mut();

        if let Some(account) = solana_accounts.get_mut(key) {
            if account.owner == *self.provider.evm_loader() { // NeonEVM accounts may be already borrowed
                return None;
            }
            Some(evm_loader::query::Value {
                owner: account.owner,
                length: account.data.len(),
                lamports: account.lamports,
                executable: account.executable,
                rent_epoch: account.rent_epoch,
                offset: data_offset,
                data: evm_loader::query::clone_chunk(&account.data, data_offset, data_len),
            })
        }
        else{
            None
        }
    }


    fn solana_accounts_space(&self, address: &H160) -> (usize, usize) {
        let account_space = {
            self.ethereum_account_map_or(address, 0, |a| a.info.data_len())
        };

        let contract_space = {
            self.ethereum_contract_map_or(address,
                                          0,
                                          |a| {
                                              EthereumContract::SIZE
                                                  + a.extension.code.len()
                                                  + a.extension.valids.len()
                                                  + a.extension.storage.len()
                                          })
        };

        (account_space, contract_space)
    }

    fn storage(&self, address: &H160, index: &U256) -> U256 {
        if *index < U256::from(STORAGE_ENTIRIES_IN_CONTRACT_ACCOUNT) {
            let index: usize = index.as_usize() * 32;
            return self.ethereum_contract_map_or(
                address,
                None,
                |c| Some(U256::from(&c.extension.storage[index..index+32])))
                .unwrap_or_else(U256::zero);
        }

        let (solana_address, _) = self.get_storage_address(address, index);
        let mut accounts = self.solana_accounts.borrow_mut();
        let account = accounts.get_mut(&solana_address)
            .unwrap_or_else(|| panic!("Account {} - storage account not found", solana_address));

        if &account.owner == self.program_id() {
            let acc_info = account_info(&solana_address, account);
            let storage = EthereumStorage::from_account(self.program_id(), &acc_info).unwrap();
            return storage.value
        }

        if solana_program::system_program::check_id(&account.owner) {
            return U256::zero()
        }

        panic!("Account {} - expected system or program owned", solana_address);
    }

    fn generation(&self, address: &H160) -> u32 {
        self.ethereum_contract_map_or(
            address,
            0_u32,
            |c| c.generation
        )
    }

    fn valids(&self, address: &H160) -> Vec<u8> {
        self.ethereum_contract_map_or(address,
                                      Vec::new(),
                                      |c| c.extension.valids.to_vec()
        )
    }

    fn token_mint(&self) -> &Pubkey { &self.token_mint }

    fn block_hash(&self, number: evm::U256) -> evm::H256 {
        if !self.create_sol_acc_if_not_exists(&recent_blockhashes::ID) {
            warn!("Failed to create/find recent_blockhashed account");
            return evm::H256::default();
        }

        let mut solana_accounts = self.solana_accounts.borrow_mut();

        if let Some(account) = solana_accounts.get_mut(&recent_blockhashes::ID) {
            let info = account_info(&recent_blockhashes::ID, account);
            let slot_hash_data = info.data.borrow();
            let clock_slot = self.clock.slot;
            if number >= clock_slot.into() {
                return H256::default();
            }
            let offset: usize = (8 + (clock_slot - 1 - number.as_u64()) * 40).try_into().unwrap();
            if offset + 32 > slot_hash_data.len() {
                return H256::default();
            }
            return H256::from_slice(&slot_hash_data[offset..][..32]);
        }
        else {
            evm::H256::default()
        }
    }

    fn chain_id(&self) -> u64 { self.chain_id }
}
