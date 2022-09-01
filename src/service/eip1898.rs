use {
    crate::{
        neon::{ account_storage::EmulatorAccountStorage, provider::DbProvider, TracerCore, Result },
        syscall_stubs::Stubs,
        v1::{
            geth::types::trace::{H160T, U256T},
            types::{ BlockNumber, EthCallObject },
        },
    },
    evm::{ ExitReason, U256 },
    evm_loader::{
        account_storage::AccountStorage,
        executor::Machine,
    },
    jsonrpsee::{ proc_macros::rpc, types::Error },
    tracing::{debug, instrument},
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

impl TracerCore {
    fn create_account_storage(&self, tag: BlockNumber) -> Result<EmulatorAccountStorage<DbProvider>> {
        let block_number = self.get_block_number(tag)?;
        let provider = self.db_provider();
        let syscall_stubs = Stubs::new(&provider, block_number)?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);
        let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));

        Ok(account_storage)
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

        let account_storage = self.create_account_storage(tag)?;
        let caller_id = object.from.map(|v| v.0).unwrap_or_default();
        let mut executor = Machine::new(caller_id, &account_storage)
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
        let account_storage = self.create_account_storage(tag)?;
        Ok(U256T(account_storage.storage(&contract_id.0, &index.0)))
    }

    #[instrument]
    fn eth_get_balance(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        let account_storage = self.create_account_storage(tag)?;
        Ok(U256T(account_storage.balance(&address.0)))
    }

    #[instrument]
    fn eth_get_code(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<String> {
        let account_storage = self.create_account_storage(tag)?;
        let code = account_storage.code(&address.0);
        Ok(format!("0x{}", hex::encode(code)))
    }

    #[instrument]
    fn eth_get_transaction_count(
        &self,
        account_id: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        let account_storage = self.create_account_storage(tag)?;
        Ok(U256T(account_storage.nonce(&account_id.0)))
    }
}
