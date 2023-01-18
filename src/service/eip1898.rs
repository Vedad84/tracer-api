use {
    async_trait::async_trait,
    jsonrpsee::{ proc_macros::rpc, types::Error },
    log::*,
    parity_bytes::ToPretty,
    crate::{
        metrics,
        neon::{tracer_core::TracerCore,  Result},
        types::{H160T, U256T, BlockNumber, EthCallObject},
    },
};

#[rpc(server)]
#[async_trait]
pub trait EIP1898 {
    #[method(name = "eth_call")]
    async fn eth_call(&self, object: EthCallObject,  tag: BlockNumber) -> Result<String>;
    #[method(name = "eth_getStorageAt")]
    async fn eth_get_storage_at(&self, address: H160T, index: U256T, tag: BlockNumber) -> Result<U256T>;
    #[method(name = "eth_getBalance")]
    async fn eth_get_balance(&self, address: H160T, tag: BlockNumber) -> Result<U256T>;
    #[method(name = "eth_getCode")]
    async fn eth_get_code(&self, address: H160T, tag: BlockNumber) -> Result<String>;
    #[method(name = "eth_getTransactionCount")]
    async fn eth_get_transaction_count(&self, address: H160T, tag: BlockNumber) -> Result<U256T>;
}

#[async_trait]
impl EIP1898Server for TracerCore {
    async fn eth_call(&self, object: EthCallObject, tag: BlockNumber) -> Result<String> {

        let data = object.data.map(|v| v.0);
        let from = object.from.map(|v| v.0);
        let to = object.to.0;
        let value = object.value.map(|v| v.0);

        debug!(
            "eth_call_impl(caller={:?}, contract={:?}, data={:?}, value={:?})",
            from,
            object.to.0,
            data.as_ref().map(|vec| hex::encode(&vec)),
            &value,
        );

        let tout = std::time::Duration::new(10, 0);

        let started = metrics::report_incoming_request("eth_call");
        let slot = self.get_block_number(tag)?;
        let result = self.neon_cli.emulate(from, to, value, data, slot, &tout).await;
        metrics::report_request_finished(started, "eth_call", result.is_ok());

        if let Err(err) = result {
            warn!("eth_call failed: {:?}", err);
            Err(Error::Custom("Internal server error".to_string()))
        } else {
            result
        }
    }

    async fn eth_get_storage_at(&self, address: H160T,  index: U256T, tag: BlockNumber) -> Result<U256T> {
        debug!("eth_get_storage_at_impl({:?}, {:?}, {:?})", address.0.to_hex(), index.0.to_string(), tag);

        let tout = std::time::Duration::new(10, 0);

        let started = metrics::report_incoming_request("eth_getStorageAt");
        let slot = self.get_block_number(tag)?;
        let value = self.neon_cli.get_storage_at(address.0, index.0, slot, &tout).await;
        metrics::report_request_finished(started, "eth_getStorageAt", value.is_ok());

        match value {
            Ok(v) => Ok(U256T::from(v)),
            Err(e) => {
                warn!("eth_get_storage_at failed: {:?}", e);
                Err(Error::Custom("Internal server error".to_string()))
            }
        }
    }

    async fn eth_get_balance(&self, address: H160T, tag: BlockNumber) -> Result<U256T> {
        debug!("eth_get_balance_impl({:?}, {:?})", address.0.to_hex(), tag);

        let tout = std::time::Duration::new(10, 0);

        let started = metrics::report_incoming_request("eth_getBalance");
        let slot = self.get_block_number(tag)?;
        let balance = self.neon_cli.get_balance(address.0, slot, &tout).await;
        metrics::report_request_finished(started, "eth_getBalance", balance.is_ok());

        match balance {
            Ok(v) => Ok(U256T::from(v)),
            Err(e) => {
                warn!("eth_get_balance failed: {:?}", e);
                Err(Error::Custom("Internal server error".to_string()))
            }
        }
    }

    async fn eth_get_code(&self, address: H160T, tag: BlockNumber) -> Result<String> {
        debug!("eth_get_code_impl({:?}, {:?})", address.0.to_hex(), tag);

        let tout = std::time::Duration::new(10, 0);

        let started = metrics::report_incoming_request("eth_getCode");
        let slot = self.get_block_number(tag)?;
        let code = self.neon_cli.get_code(address.0, slot, &tout).await;
        metrics::report_request_finished(started, "eth_getCode", code.is_ok());

        if let Err(err) = code {
            warn!("eth_get_code failed: {:?}", err);
            Err(Error::Custom("Internal server error".to_string()))
        } else {
            code
        }
    }

    async fn eth_get_transaction_count(&self, address: H160T, tag: BlockNumber) -> Result<U256T> {
        debug!("eth_get_transaction_count_impl({:?}, {:?})", address.0.to_hex(), tag);

        let tout = std::time::Duration::new(10, 0);

        let started = metrics::report_incoming_request("eth_getTransactionCount");
        let slot = self.get_block_number(tag)?;
        let count = self.neon_cli.get_trx_count(address.0, slot, &tout).await;
        metrics::report_request_finished(started, "eth_getTransactionCount", count.is_ok());

        match count {
            Ok(v) => Ok(U256T::from(v)),
            Err(e) => {
                warn!("eth_transaction_count failed: {:?}", e);
                Err(Error::Custom("Internal server error".to_string()))
            }
        }
    }
}
