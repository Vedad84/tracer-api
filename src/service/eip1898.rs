use {
    anyhow::bail,
    arrayref::array_ref,
    crate::{
        neon::{ account_storage::EmulatorAccountStorage, provider::DbProvider, TracerCore },
        service::Result,
        syscall_stubs::Stubs,
        v1::{
            geth::types::trace::{H160T, H256T, U256T},
            types::{ BlockNumber, EthCallObject },
        },
    },
    evm::{ ExitReason, U256 },
    evm_loader::{
        account_storage::AccountStorage,
        executor::Machine,
    },
    jsonrpsee::{ proc_macros::rpc, types::Error },
    tracing::{debug, info, instrument, warn},
    tokio::task::block_in_place,
    web3::types::BlockId,
};

#[rpc(server)]
pub trait EIP1898 {
    #[method(name = "eth_call")]
    fn eth_call(
        &self,
        object: EthCallObject,
        tag: BlockNumber,
    ) -> Result<String>;

    #[method(name = "eth_getStorageAt")]
    fn eth_get_storage_at(
        &self,
        contract_id: H160T,
        index: U256T,
        tag: BlockNumber,
    ) -> Result<U256T>;

    #[method(name = "eth_getBalance")]
    fn eth_get_balance(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<U256T>;

    #[method(name = "eth_getCode")]
    fn eth_get_code(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<String>;

    #[method(name = "eth_getTransactionCount")]
    fn eth_get_transaction_count(
        &self,
        contract_id: H160T,
        tag: BlockNumber,
    ) -> Result<U256T>;
}

fn convert_h256(inp: H256T) -> web3::types::H256 {
    let bytes = array_ref![inp.0.as_bytes(), 0, 32];
    web3::types::H256::from(bytes)
}

impl TracerCore {
    fn get_block_number(&self, tag: BlockNumber) -> Result<u64> {
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
            _ => {
                Err(Error::Custom(format!("Unsupported block tag")))
            }
        }
    }
}

impl EIP1898Server for TracerCore {
    #[instrument]
    fn eth_call(
        &self,
        object: EthCallObject,
        tag: BlockNumber,
    ) -> Result<String> {
        let data = object.data.map(|v| v.0);
        let value = object.value.map(|v| v.0);

        let block_number = self.get_block_number(tag)?;
        let provider = DbProvider::new(
            self.db_client.clone(),
            self.evm_loader,
        );

        let caller_id = object.from.map(|v| v.0).unwrap_or_default();
        let syscall_stubs = Stubs::new(&provider, block_number)?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

        let storage = EmulatorAccountStorage::new(provider, Some(block_number));
        let mut executor = Machine::new(caller_id, &storage)
            .map_err(|err| Error::Custom(format!("Failed to create Machine: {:?}", err)))?;

        // u64::MAX is too large, remix gives this error:
        // Gas estimation errored with the following message (see below).
        // Number can only safely store up to 53 bits
        let gas_limit = U256::from(object.gas.map(|v| v.0).unwrap_or_else(|| 50_000_000u32.into()));

        debug!(
            "call_begin(caller_id={:?}, contract_id={:?}, data={:?}, value={:?})",
            caller_id,
            object.to.0,
            data.as_ref().map(|vec| hex::encode(&vec)),
            &value,
        );

        executor.call_begin(
            caller_id,
            object.to.0,
            data.unwrap_or_default(),
            value.unwrap_or_default(),
            gas_limit,
            object.gasprice.map(|v| v.0).unwrap_or_default(),
        ).map_err(|err| Error::Custom(format!("Failed to execute transaction: {:?}", err)))?;

        let (result, exit_reason) = match executor.execute_n_steps(100_000) {
            Ok(()) => return Err(Error::Custom("bad account kind".to_string())),
            Err(result) => result,
        };

        let status = match exit_reason {
            ExitReason::Succeed(_) => "succeed".to_string(),
            ExitReason::Error(_) => "error".to_string(),
            ExitReason::Revert(_) => "revert".to_string(),
            ExitReason::Fatal(_) => "fatal".to_string(),
            ExitReason::StepLimitReached => return Err(Error::Custom("Step limit reached".to_string())),
        };

        if status.eq("succeed") {
            return Ok(format!("0x{}", hex::encode(&result)));
        }

        Ok("0x".to_string())
    }

    #[instrument]
    fn eth_get_storage_at(
        &self,
        contract_id: H160T,
        index: U256T,
        tag: BlockNumber,
    ) -> Result<U256T> {

        let block_number = self.get_block_number(tag)?;
        let provider = DbProvider::new(
            self.db_client.clone(),
            self.evm_loader,
        );

        let syscall_stubs = Stubs::new(&provider, block_number)?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

        let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));
        Ok(U256T(account_storage.storage(&contract_id.0, &index.0)))
    }

    #[instrument]
    fn eth_get_balance(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {

        let block_number = self.get_block_number(tag)?;
        let provider = DbProvider::new(
            self.db_client.clone(),
            self.evm_loader,
        );

        let syscall_stubs = Stubs::new(&provider, block_number)?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

        let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));
        Ok(U256T(account_storage.balance(&address.0)))
    }

    #[instrument]
    fn eth_get_code(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<String> {
        let block_number = self.get_block_number(tag)?;
        let provider = DbProvider::new(
            self.db_client.clone(),
            self.evm_loader,
        );

        let syscall_stubs = Stubs::new(&provider, block_number)?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

        let code = EmulatorAccountStorage::new(provider, Some(block_number))
            .code(&address.0);
        Ok(format!("0x{}", hex::encode(code)))
    }

    #[instrument]
    fn eth_get_transaction_count(
        &self,
        account_id: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        let block_number = self.get_block_number(tag)?;
        let provider = DbProvider::new(
            self.db_client.clone(),
            self.evm_loader,
        );

        let syscall_stubs = Stubs::new(&provider, block_number)?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);

        let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));
        Ok(U256T(account_storage.nonce(&account_id.0)))
    }
}