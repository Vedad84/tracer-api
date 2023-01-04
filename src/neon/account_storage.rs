use {
    crate::neon::account_info,
    evm::{H160, H256, U256},
    evm_loader::{
        account_storage::{AccountStorage},
        account::{ ACCOUNT_SEED_VERSION, ether_contract, EthereumAccount, EthereumStorage },
        executor::{ OwnedAccountInfo, OwnedAccountInfoPartial },
        config::STORAGE_ENTRIES_IN_CONTRACT_ACCOUNT,
        precompile::is_precompile_address,
    },
    solana_program::sysvar::recent_blockhashes,
    solana_sdk::{
        account::Account,
        pubkey,
        pubkey::Pubkey,
        sysvar::{
            clock::Clock,
            Sysvar,
        },
    },
    std::{
        borrow::BorrowMut,
        cell::RefCell,
        collections::HashMap,
        convert::TryInto,
        env,
        str::FromStr,
    },
    super::provider::Provider,
    tracing::{ debug, info, warn, error },
};

const FAKE_OPERATOR: Pubkey = pubkey!("neonoperator1111111111111111111111111111111");

struct SolanaAccount {
    account: Account,
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
        let slot = block_number.unwrap_or_else(|| {
            if let Ok(slot) = provider.get_slot() {
                slot
            } else {
                warn!("Get slot error");
                0
            }
        });

        let timestamp = if let Ok(timestamp) = provider.get_block_time(slot) {
            timestamp
        } else {
            warn!("Get timestamp error");
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
            provider,
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

        if is_precompile_address(address) {
            return true;
        }

        let mut ether_accounts = self.ethereum_accounts.borrow_mut();

        if !ether_accounts.contains_key(address) {
            let (key, _) = Pubkey::find_program_address(&[&[ACCOUNT_SEED_VERSION], address.as_bytes()],  self.provider.evm_loader());

            let result = self.provider.get_account_at_slot(&key, self.block_number)
                .unwrap_or_default()
                .map_or(false, |account| {
                    ether_accounts.insert(*address, SolanaAccount{ account, key });
                    true
                });

            if !result {
                warn!("Failed to load solana account {} for ethereum account {}", key, address);
            }

            return result
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
        if !self.create_acc_if_not_exists(address) {
            warn!("Failed to create/find ethereum account {:?}", address);
            return default
        }

        let mut accounts = self.ethereum_accounts.borrow_mut();

        if let Some( solana) = accounts.get_mut(address) {
            let info = account_info(&solana.key, &mut solana.account);

            let ethereum_account_res = EthereumAccount::from_account(
                self.provider.evm_loader(),
                &info
            );

            match ethereum_account_res {
                Ok(ethereum_account) => f(&ethereum_account),
                Err(err) => {
                    error!("Failed to create EthereumAccount: {:?}", err);
                    default
                }
            }
        } else {
            default
        }
    }

    fn ethereum_contract_map_or<F, D>(&self, address: &H160, default: D, f: F) -> D
       where
            F: FnOnce(ether_contract::ContractData) -> D
    {
        if !self.create_acc_if_not_exists(address) {
            warn!("Failed to create/find ethereum contract {:?}", address);
            return default
        }

        let mut accounts = self.ethereum_accounts.borrow_mut();
        if let Some(account) = accounts.get_mut(address) {
            let info = account_info(&account.key, &mut account.account);
            let ether_account_res = EthereumAccount::from_account(
                self.provider.evm_loader(),
                &info
            );

            match ether_account_res {
                Ok(ethereum_account) =>
                    if let Some(contract) = ethereum_account.contract_data() {
                        f(contract)
                    } else {
                        error!("EthereumAccount {}: is not contract account", address);
                        default
                    }
                Err(err) => {
                    error!("Failed to create EthereumAccount: {:?}", err);
                    default
                },
            }
        } else {
            default
        }
    }
}

impl<P: Provider> AccountStorage for EmulatorAccountStorage<P> {

    fn program_id(&self) -> &Pubkey {
        info!("program_id");
        self.provider.evm_loader()
    }

    fn operator(&self) -> &Pubkey {
        info!("operator");
        &FAKE_OPERATOR
    }

    fn balance(&self, address: &H160) -> U256 {
        info!("balance {}", address);

        self.ethereum_account_map_or(address, U256::zero(), |a| a.balance)
    }

    fn block_number(&self) -> U256 {
        info!("block_number");
        self.block_number.into()
    }

    fn block_timestamp(&self) -> U256 {
        info!("block_timestamp");
        self.block_timestamp.into()
    }


    fn nonce(&self, address: &H160) -> U256 {
        info!("nonce {}", address);

        self.ethereum_account_map_or(address, 0_u64, |a| a.trx_count).into()
    }

    fn code(&self, address: &H160) -> Vec<u8> {
        info!("code {}", address);

        self.ethereum_contract_map_or(
            address,
            Vec::new(),
            |c| c.code().to_vec()
        )
    }

    fn code_hash(&self, address: &H160) -> H256 {
        info!("code_hash {}", address);

        self.ethereum_contract_map_or(
            address,
            H256::default(),
            |c| evm_loader::utils::keccak256_h256(&c.code())
        )
    }

    fn code_size(&self, address: &H160) -> usize {
        info!("code_size {}", address);
        self.ethereum_account_map_or(address, 0, |a| a.code_size as usize)
    }

    fn exists(&self, address: &H160) -> bool {
        info!("exists {}", address);

        self.create_acc_if_not_exists(address);

        let accounts = self.ethereum_accounts.borrow();
        accounts.contains_key(address)
    }

    fn solana_account_space(&self, address: &H160) -> Option<usize> {
        self.ethereum_account_map_or(address, None, |account| Some(account.info.data_len()))
    }

    fn storage(&self, address: &H160, index: &U256) -> U256 {
        info!("storage {} -> {}", address, index);

        let value = if *index < U256::from(STORAGE_ENTRIES_IN_CONTRACT_ACCOUNT) {
            let index: usize = index.as_usize() * 32;
            self.ethereum_contract_map_or(
                address,
                U256::zero(),
                |c| U256::from_big_endian(&c.storage()[index..index+32])
            )
        } else {
            #[allow(clippy::cast_possible_truncation)]
            let subindex = (*index & U256::from(0xFF)).as_u64() as u8;
            let index = *index & !U256::from(0xFF);

            let solana_address = EthereumStorage::solana_address(self, address, &index);

            if !self.create_sol_acc_if_not_exists(&solana_address) {
                warn!("storage: failed to read solana account {}", solana_address);
                return U256::zero();
            }

            let mut accounts = self.solana_accounts.borrow_mut();
            let account = accounts.get_mut(&solana_address)
                .unwrap_or_else(|| panic!("Account {} - storage account not found", solana_address));

            if solana_sdk::system_program::check_id(&account.owner) {
                debug!("read storage system owned");
                U256::zero()
            } else {
                let account_info = account_info(&solana_address, account);
                let storage = EthereumStorage::from_account(self.provider.evm_loader(), &account_info).unwrap();
                if (storage.address != *address) || (storage.index != index) || (storage.generation != self.generation(address)) {
                    debug!("storage collision");
                    U256::zero()
                } else {
                    storage.get(subindex)
                }
            }
        };

        debug!("Storage read {:?} -> {} = {}", address, index, value);

        value
    }

    fn generation(&self, address: &H160) -> u32 {
        info!("generation {}", address);
        let value = self.ethereum_account_map_or(
            address,
            0_u32,
            |c| c.generation
        );

        info!("account generation {:?} - {:?}", address, value);
        value
    }

    fn valids(&self, address: &H160) -> Vec<u8> {
        info!("valids {}", address);

        self.ethereum_contract_map_or(
            address,
            Vec::new(),
            |c| c.valids().to_vec()
        )
    }

    fn neon_token_mint(&self) -> &Pubkey { &self.token_mint }

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

            let offset = (8 + (clock_slot - 1 - number.as_u64()) * 40).try_into();
            return match offset {
                Ok(offset) => {
                    if offset + 32 > slot_hash_data.len() {
                        return H256::default();
                    }
                    H256::from_slice(&slot_hash_data[offset..][..32])
                }
                Err(err) => {
                    error!("Failed calculate offset: {}", err);
                    evm::H256::default()
                }
            }
        }
        else {
            evm::H256::default()
        }
    }

    fn chain_id(&self) -> u64 {
        info!("chain_id");

        self.chain_id
    }

    fn clone_solana_account(&self, address: &Pubkey) -> OwnedAccountInfo {
        info!("clone_solana_account {}", address);

        if address == &FAKE_OPERATOR {
            OwnedAccountInfo {
                key: FAKE_OPERATOR,
                is_signer: true,
                is_writable: false,
                lamports: 100 * 1_000_000_000,
                data: vec![],
                owner: solana_sdk::system_program::ID,
                executable: false,
                rent_epoch: 0,
            }
        } else {
            if !self.create_sol_acc_if_not_exists(address) {
                warn!("clone_solana_account: Failed to create solana account {}. Will use default", address);
            }

            let mut accounts = self.solana_accounts.borrow_mut();
            let mut default_account = Account::default();
            let account = accounts.get_mut(address).unwrap_or(&mut default_account);
            let info = account_info(
                address,
                account
            );

            OwnedAccountInfo::from_account_info(&info)
        }
    }

    fn clone_solana_account_partial(&self, address: &Pubkey, offset: usize, len: usize) -> Option<OwnedAccountInfoPartial> {
        info!("clone_solana_account_partial {}", address);

        let account = self.clone_solana_account(address);

        Some(OwnedAccountInfoPartial {
            key: account.key,
            is_signer: account.is_signer,
            is_writable: account.is_writable,
            lamports: account.lamports,
            data: account.data.get(offset .. offset + len).map(<[u8]>::to_vec)?,
            data_offset: offset,
            data_total_len: account.data.len(),
            owner: account.owner,
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        })
    }
}
