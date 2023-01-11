use {
    async_trait::async_trait,
    bollard::{
        container::LogOutput as DockerLogOutput,
        exec::StartExecResults,
    },
    crate::{
        metrics,
        neon::{ account_storage::EmulatorAccountStorage, provider::DbProvider, tracer_core::TracerCore,
                Result},
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
