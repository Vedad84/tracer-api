use {
    crate::{
        service::{ Result, ServerImpl },
        v1::{
            geth::types::trace::{H160T, H256T, U256T},
            types::{ BlockNumber, EthCallObject },
        },
    },
    jsonrpsee::{ proc_macros::rpc, types::Error },
    tracing::{instrument},
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

impl EIP1898Server for ServerImpl {
    #[instrument]
    fn eth_call(
        &self,
        object: EthCallObject,
        tag: BlockNumber,
    ) -> Result<String> {
        self.tracer_core.eth_call(
            object.from.map(|v| v.0),
            object.to.0,
            object.gas.map(|v| v.0),
            object.gasprice.map(|v| v.0),
            object.value.map(|v| v.0),
            object.data.map(|v| v.0),
            tag,
        )
            .map_err(|err| Error::Custom(err.to_string()))
    }

    #[instrument]
    fn eth_get_storage_at(
        &self,
        contract_id: H160T,
        index: U256T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        self.tracer_core.get_storage_at(&contract_id, &index, tag)
            .map_err(|err| Error::Custom(err.to_string()))
    }

    #[instrument]
    fn eth_get_balance(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        self.tracer_core.get_balance(&address, tag)
            .map_err(|err|Error::Custom(err.to_string()))
    }

    #[instrument]
    fn eth_get_code(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<String> {
        self.tracer_core.get_code(&address, tag)
            .map_err(|err|Error::Custom(err.to_string()))
    }

    #[instrument]
    fn eth_get_transaction_count(
        &self,
        account_id: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        self.tracer_core.get_transaction_count(&account_id, tag)
            .map_err(|err|Error::Custom(err.to_string()))
    }
}