use {
    arrayref::array_ref,
    solana_sdk::pubkey::Pubkey,
    std::{fmt, sync::Arc,},
    tokio::task::block_in_place,
    web3::{ transports::Http, types::BlockId, Web3 },
    tracing::info,
    log::*,
    parity_bytes::ToPretty,
    evm_loader::account_storage::{AccountStorage},
    crate::{
        db::DbClient,
        evm_runtime::EVMRuntime,
        neon::provider::DbProvider,
        types::{H256T, U256T, H160T, BlockNumber, EthCallObject},
        neon::account_storage::EmulatorAccountStorage,
        syscall_stubs::Stubs,
    },
    super::{Error, Result,  neon_cli::NeonCli},
};

fn convert_h256(inp: H256T) -> web3::types::H256 {
    let bytes = array_ref![inp.0.as_bytes(), 0, 32];
    web3::types::H256::from(bytes)
}


#[derive(Clone)]
pub struct TracerCore {
    evm_loader: Pubkey,
    tracer_db_client: Arc<DbClient>,
    indexer_db_client: Arc<DbClient>,
    web3: Arc<Web3<Http>>,
    neon_cli: NeonCli,
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


impl TracerCore {
    pub fn new(
        evm_loader: Pubkey,
        tracer_db_client: Arc<DbClient>,
        indexer_db_client: Arc<DbClient>,
        web3: Arc<Web3<Http>>,
        evm_runtime: Arc<EVMRuntime>,
    ) -> Self {

        Self {
            evm_loader,
            tracer_db_client,
            indexer_db_client,
            web3,
            neon_cli: NeonCli::new(evm_runtime),
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
                    .ok_or_else(|| Error::Custom(format!("Failed to obtain block number for hash: {}", hash_str)))?
                    .number
                    .ok_or_else(|| Error::Custom(format!("Failed to obtain block number for hash: {}", hash_str)))?
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
                Err(Error::Custom("Unsupported block tag".to_string()))
            }
        }
    }

    fn create_account_storage(&self, tag: BlockNumber) -> Result<EmulatorAccountStorage<DbProvider>> {
        let block_number = self.get_block_number(tag)?;
        let provider = self.tracer_db_provider();
        let syscall_stubs = Stubs::new(&provider, block_number)?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);
        let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));

        Ok(account_storage)
    }

    pub async fn eth_call_impl(&self,  object: EthCallObject, tag: BlockNumber) -> Result<String> {
        let data = object.data.map(|v| v.0);
        let caller_id = object.from.map(|v| v.0);
        let contract_id = object.to.0;
        let value = object.value.map(|v| v.0);

        debug!(
            "eth_call_impl(caller_id={:?}, contract_id={:?}, data={:?}, value={:?})",
            caller_id,
            object.to.0,
            data.as_ref().map(|vec| hex::encode(&vec)),
            &value,
        );

        let slot = self.get_block_number(tag)?;
        let tout = std::time::Duration::new(10, 0);

        self.neon_cli.emulate(caller_id, contract_id, value, data,slot, &tout).await
    }

    pub fn eth_get_storage_at_impl(
        &self,
        contract_id: H160T,
        index: U256T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        debug!("eth_get_storage_at_impl({:?}, {:?}, {:?})", contract_id.0.to_hex(), index.0.to_string(), tag);
        let account_storage = self.create_account_storage(tag)?;
        Ok(U256T(account_storage.storage(&contract_id.0, &index.0)))
    }

    pub fn eth_get_balance_impl(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        debug!("eth_get_balance_impl({:?}, {:?})", address.0.to_hex(), tag);
        let account_storage = self.create_account_storage(tag)?;
        Ok(U256T(account_storage.balance(&address.0)))
    }

    pub fn eth_get_code_impl(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<String> {
        debug!("eth_get_code_impl({:?}, {:?})", address.0.to_hex(), tag);
        let account_storage = self.create_account_storage(tag)?;
        let code = account_storage.code(&address.0);
        Ok(format!("0x{}", hex::encode(code)))
    }

    pub fn eth_get_transaction_count_impl(
        &self,
        account_id: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        debug!("eth_get_transaction_count_impl({:?}, {:?})", account_id.0.to_hex(), tag);
        let account_storage = self.create_account_storage(tag)?;
        Ok(U256T(account_storage.nonce(&account_id.0)))
    }
}
