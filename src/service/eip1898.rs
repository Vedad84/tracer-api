use {
    async_trait::async_trait,
    bollard::{
        container::LogOutput as DockerLogOutput,
        exec::StartExecResults,
    },
    crate::{
        metrics,
        neon::{ account_storage::EmulatorAccountStorage, provider::DbProvider, TracerCore, Result },
        syscall_stubs::Stubs,
        v1::{
            geth::types::trace::{H160T, U256T},
            types::{ BlockNumber, EthCallObject },
        },
    },
    futures_core::stream::Stream,
    futures_util::{ pin_mut, stream::StreamExt },
    jsonrpsee::{ proc_macros::rpc, types::Error },
    log::*,
    parity_bytes::ToPretty,
    phf::phf_map,
    serde::Serialize,
    serde_json::Value as JSONValue,
    std::ops::Deref,
    tokio::io::AsyncWriteExt,
    evm::{ ExitReason, U256 },
    evm_loader::{
        account_storage::AccountStorage,
        executor::Machine,
    },
};

#[rpc(server)]
#[async_trait]
pub trait EIP1898 {
    #[method(name = "eth_call")]
    async fn eth_call(
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

fn decode_error_message(reason: &str) -> Option<String> {
    return ETHEREUM_ERROR_MAP.get(reason).map(|s| s.to_string())
}

fn decode_fatal_message(reason: &str) -> Option<String> {
    return ETHEREUM_FATAL_ERROR_MAP.get(reason).map(|s| s.to_string())
}

impl TracerCore {
    fn create_account_storage(&self, tag: BlockNumber) -> Result<EmulatorAccountStorage<DbProvider>> {
        let block_number = self.get_block_number(tag)?;
        let provider = self.tracer_db_provider();
        let syscall_stubs = Stubs::new(&provider, block_number)?;
        solana_sdk::program_stubs::set_syscall_stubs(syscall_stubs);
        let account_storage = EmulatorAccountStorage::new(provider, Some(block_number));

        Ok(account_storage)
    }

    fn decode_result(&self, obj: &serde_json::Map<String, JSONValue>) -> Result<String> {
        return if let Some(data) = obj.get("result") {
            if let JSONValue::String(string) = data {
                Ok(string.clone())
            } else {
                warn!("result in emulator output is not a string");
                Err(INTERNAL_SERVER_ERROR())
            }
        } else {
            warn!("result is absend in emulator output");
            Err(INTERNAL_SERVER_ERROR())
        }
    }

    fn decode_revert_message(&self, data: &String) -> Result<Option<String>> {
        let data_len = data.len();
        if data_len == 0 {
            return Ok(None);
        }

        if data_len < 8 {
            warn!("Too less bytes to decode revert signature: {data_len}, data: 0x{data}");
            return Err(INTERNAL_SERVER_ERROR());
        }

        if &data[..8] == "4e487b71" { // keccak256("Panic(uint256)")
            return Ok(None)
        }

        if &data[..8] == "08c379a0" { // keccak256("Error(string)")
            warn!("Failed to decode revert_message, unknown revert signature: {}", data[..8].to_string());
            return Ok(None)
        }

        if data_len < 8 + 64 {
            warn!("Too less bytes to decode revert msg offset: {data_len}, data: 0x{data}");
            return Err(INTERNAL_SERVER_ERROR())
        }

        let offset = usize::from_str_radix(&data[8..(8 + 64)], 16)
            .map_err(|err| {
                warn!("Failed to parse rever reason offset: {data_len}, data: 0x{data}: {err:?}");
                INTERNAL_SERVER_ERROR()
            })?;
        let offset = offset * 2;

        if data_len < 8 + offset + 64 {
            warn!("Too less bytes to decode revert msg len: {data_len}, data: 0x{data}");
            return Err(INTERNAL_SERVER_ERROR());
        }

        let length = usize::from_str_radix(&data[(8 + offset)..(8 + offset + 64)], 16)
            .map_err(|err| {
                warn!("Failed to parse revert reason length: {data_len}, data: 0x{data}: {err:?}");
                INTERNAL_SERVER_ERROR()
            })?;
        let length = length * 2;

        if data_len < 8 + offset + 64 + length {
            warn!("Too less bytes to decode revert msg: {data_len}, data: 0x{data}");
            return Err(INTERNAL_SERVER_ERROR());
        }

        let message_bytes = hex::decode(&data[(8 + offset + 64)..(8 + offset + 64 + length)])
            .map_err(|err| {
                warn!("Failed to decode revert from hex length: {data_len}, data: 0x{data}: {err:?}");
                INTERNAL_SERVER_ERROR()
            })?;

        let message = std::str::from_utf8(&message_bytes).map_err(|err| {
            warn!("Failed to decode UTF-8 from revert message bytes: {data_len}, data: 0x{data}: {err:?}");
            INTERNAL_SERVER_ERROR()
        })?;

        Ok(Some(message.to_string()))
    }

    fn decode_revert_result(&self, obj: &serde_json::Map<String, JSONValue>) -> Result<String> {
        let revert_data = self.decode_result(obj)?;
        let error =
            if let Some(result_value) = self.decode_revert_message(&revert_data)? {
                EthereumError {
                    code: 3,
                    message: Some(format!("execution reverted: {}", result_value)),
                    data: Some(format!("0x{}", revert_data)),
                }
            } else {
                EthereumError {
                    code: 3,
                    message: Some(format!("execution reverted")),
                    data: Some(format!("0x{}", revert_data)),
                }
            };

        Ok(serde_json::to_string(&error).map_err(|err| {
            warn!("Failed to serialize error message: {:?}", err);
            INTERNAL_SERVER_ERROR()
        })?)
    }

    fn decode_error_result(&self, exit_status: &String, obj: &serde_json::Map<String, JSONValue>) -> Result<String> {
        let error = if let Some(reason) = obj.get("exit_reason") {
            match reason {
                JSONValue::String(reason) => {
                    EthereumError {
                        code: 3,
                        message: Some(format!("execution finished with error: {reason}")),
                        data: None
                    }
                },
                JSONValue::Object(obj) => {
                    let mut error: Option<String> = None;
                    if let Some(err) = reason.get("Error") {
                        error = decode_error_message(err.to_string().as_str());
                    }

                    if error.is_none() {
                        if let Some(fatal) = reason.get("Fatal") {
                            error = decode_fatal_message(fatal.to_string().as_str());
                        }
                    }

                    if let Some(error) = error {
                        EthereumError {
                            code: 3,
                            message: Some(format!("execution finished with error: {error}")),
                            data: None
                        }
                    } else {
                        EthereumError {
                            code: 3,
                            message: Some(exit_status.clone()),
                            data: None
                        }
                    }
                },
                _ => {
                    EthereumError {
                        code: 3,
                        message: Some(exit_status.clone()),
                        data: None
                    }
                }
            }
        } else {
            EthereumError {
                code: 3,
                message: Some(exit_status.clone()),
                data: None
            }
        };

        serde_json::to_string(&error).map_err(|err| {
            warn!("Failed to serialize error {:?} to string", error);
            INTERNAL_SERVER_ERROR()
        })
    }

    fn decode_succeed_result(&self, obj: &serde_json::Map<String, JSONValue>) -> Result<String> {
        if let Some(result) = obj.get("result") {
            if let JSONValue::String(result) = result {
                return Ok(format!("0x{result}"));
            }

            warn!("Unexpected result type in JSON");
            return Err(INTERNAL_SERVER_ERROR())
        }

        Ok("0x".to_string())
    }

    fn decode_emulation_result(&self, result: JSONValue) -> Result<String> {
        return match result {
            JSONValue::Object(obj) => {
                if let Some(exit_status) = obj.get("exit_status") {
                    if let JSONValue::String(exit_status) = exit_status {
                        if exit_status == "revert" {
                            self.decode_revert_result(&obj)
                        } else if exit_status != "succeed" {
                            self.decode_error_result(exit_status, &obj)
                        } else {
                            self.decode_succeed_result(&obj)
                        }
                    } else {
                        error!("exit_status expected to be a String");
                        Err(INTERNAL_SERVER_ERROR())
                    }
                } else {
                    error!("Emulation exit_status undefined");
                    Err(INTERNAL_SERVER_ERROR())
                }

            },
            _ => {
                error!("Emulation result expected to be JSON Object");
                Err(INTERNAL_SERVER_ERROR())
            },
        }
    }

    async fn eth_call_impl(
        &self,
        object: EthCallObject,
        tag: BlockNumber,
    ) -> Result<String> {

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

        let block_number = self.get_block_number(tag)?;
        let tout = std::time::Duration::new(10, 0);

        return match self.evm_runtime.run_emulate_on_slot(
            caller_id, contract_id, value, data,block_number, &tout
        ).await {
            Ok(json) => {
                self.decode_emulation_result(json)
            },
            Err(err) => {
                warn!("Emulation failed: {:?}", err);
                Err(INTERNAL_SERVER_ERROR())
            },
        }
    }

    fn eth_get_storage_at_impl(
        &self,
        contract_id: H160T,
        index: U256T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        debug!("eth_get_storage_at_impl({:?}, {:?}, {:?})", contract_id.0.to_hex(), index.0.to_string(), tag);
        let account_storage = self.create_account_storage(tag)?;
        Ok(U256T(account_storage.storage(&contract_id.0, &index.0)))
    }

    fn eth_get_balance_impl(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        debug!("eth_get_balance_impl({:?}, {:?})", address.0.to_hex(), tag);
        let account_storage = self.create_account_storage(tag)?;
        Ok(U256T(account_storage.balance(&address.0)))
    }

    fn eth_get_code_impl(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<String> {
        debug!("eth_get_code_impl({:?}, {:?})", address.0.to_hex(), tag);
        let account_storage = self.create_account_storage(tag)?;
        let code = account_storage.code(&address.0);
        Ok(format!("0x{}", hex::encode(code)))
    }

    fn eth_get_transaction_count_impl(
        &self,
        account_id: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        debug!("eth_get_transaction_count_impl({:?}, {:?})", account_id.0.to_hex(), tag);
        let account_storage = self.create_account_storage(tag)?;
        Ok(U256T(account_storage.nonce(&account_id.0)))
    }
}

#[async_trait]
impl EIP1898Server for TracerCore {
    async fn eth_call(
        &self,
        object: EthCallObject,
        tag: BlockNumber,
    ) -> Result<String> {
        let started = metrics::report_incoming_request("eth_call");
        let result = self.eth_call_impl(object, tag).await;
        metrics::report_request_finished(started, "eth_call", result.is_ok());
        return if let Err(err) = result {
            warn!("eth_call failed: {:?}", err);
            Err(Error::Custom("Internal server error".to_string()))
        } else {
            result
        }
    }

    fn eth_get_storage_at(
        &self,
        contract_id: H160T,
        index: U256T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        let started = metrics::report_incoming_request("eth_getStorageAt");
        let result = self.eth_get_storage_at_impl(contract_id, index, tag);
        metrics::report_request_finished(started, "eth_getStorageAt", result.is_ok());
        return if let Err(err) = result {
            warn!("eth_get_storage_at failed: {:?}", err);
            Err(Error::Custom("Internal server error".to_string()))
        } else {
            result
        }
    }

    fn eth_get_balance(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        let started = metrics::report_incoming_request("eth_getBalance");
        let result = self.eth_get_balance_impl(address, tag);
        metrics::report_request_finished(started, "eth_getBalance", result.is_ok());
        return if let Err(err) = result {
            warn!("eth_get_balance failed: {:?}", err);
            Err(Error::Custom("Internal server error".to_string()))
        } else {
            result
        }
    }

    fn eth_get_code(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<String> {
        let started = metrics::report_incoming_request("eth_getCode");
        let result = self.eth_get_code_impl(address, tag);
        metrics::report_request_finished(started, "eth_getCode", result.is_ok());
        return if let Err(err) = result {
            warn!("eth_get_code failed: {:?}", err);
            Err(Error::Custom("Internal server error".to_string()))
        } else {
            result
        }
    }

    fn eth_get_transaction_count(
        &self,
        account_id: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        let started = metrics::report_incoming_request("eth_getTransactionCount");
        let result = self.eth_get_transaction_count_impl(account_id, tag);
        metrics::report_request_finished(started, "eth_getTransactionCount", result.is_ok());
        return if let Err(err) = result {
            warn!("eth_get_transaction_count failed: {:?}", err);
            Err(Error::Custom("Internal server error".to_string()))
        } else {
            result
        }
    }
}
