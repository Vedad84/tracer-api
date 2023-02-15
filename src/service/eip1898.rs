use {
    async_trait::async_trait,
    jsonrpsee::proc_macros::rpc,
    log::*,
    crate::{
        metrics,
        data_source::DataSource,
        service::Result,
        types::{BlockNumber, geth::TransactionArgs},
    },
    evm_loader::types::Address,
    ethnum::U256,
};

#[rpc(server)]
#[async_trait]
pub trait EIP1898 {
    #[method(name = "eth_call")]
    async fn eth_call(&self, object: TransactionArgs,  tag: BlockNumber) -> Result<String>;
    #[method(name = "eth_getStorageAt")]
    async fn eth_get_storage_at(&self, address: Address, index: U256, tag: BlockNumber) -> Result<U256>;
    #[method(name = "eth_getBalance")]
    async fn eth_get_balance(&self, address: Address, tag: BlockNumber) -> Result<U256>;
    #[method(name = "eth_getCode")]
    async fn eth_get_code(&self, address: Address, tag: BlockNumber) -> Result<String>;
    #[method(name = "eth_getTransactionCount")]
    async fn eth_get_transaction_count(&self, address: Address, tag: BlockNumber) -> Result<U256>;
}

#[async_trait]
impl EIP1898Server for DataSource {
    async fn eth_call(&self, o: TransactionArgs, tag: BlockNumber) -> Result<String> {
        let started = metrics::report_incoming_request("eth_call");

        let data = o.input.map(|a| a.0);
        debug!(
            "eth_call_impl(caller={:?}, contract={:?}, data={:?}, value={:?})",
            o.from,
            o.to,
            data.as_ref().map(|vec| hex::encode(&vec)),
            o.value,
        );

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag)?;
        let result = self.neon_cli.emulate(o.from, o.to, o.value, data, slot, &tout).await;
        metrics::report_request_finished(started, "eth_call", result.is_ok());

        result
    }

    async fn eth_get_storage_at(&self, address: Address,  index: U256, tag: BlockNumber) -> Result<U256> {
        let started = metrics::report_incoming_request("eth_getStorageAt");

        debug!("eth_get_storage_at_impl({:?}, {:?}, {:?})", address, index, tag);

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag)?;
        let value = self.neon_cli.get_storage_at(address, index, slot, &tout).await;
        metrics::report_request_finished(started, "eth_getStorageAt", value.is_ok());

        value
    }

    async fn eth_get_balance(&self, address: Address, tag: BlockNumber) -> Result<U256> {
        let started = metrics::report_incoming_request("eth_getBalance");

        debug!("eth_get_balance_impl({:?}, {:?})", address, tag);

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag)?;
        let balance = self.neon_cli.get_balance(address, slot, &tout).await;
        metrics::report_request_finished(started, "eth_getBalance", balance.is_ok());

        balance
    }

    async fn eth_get_code(&self, address: Address, tag: BlockNumber) -> Result<String> {
        let started = metrics::report_incoming_request("eth_getCode");

        debug!("eth_get_code_impl({:?}, {:?})", address, tag);

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag)?;
        let code = self.neon_cli.get_code(address, slot, &tout).await;
        metrics::report_request_finished(started, "eth_getCode", code.is_ok());

        code
    }

    async fn eth_get_transaction_count(&self, address: Address, tag: BlockNumber) -> Result<U256> {
        let started = metrics::report_incoming_request("eth_getTransactionCount");

        debug!("eth_get_transaction_count_impl({:?}, {:?})", address, tag);

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag)?;
        let count = self.neon_cli.get_trx_count(address, slot, &tout).await;
        metrics::report_request_finished(started, "eth_getTransactionCount", count.is_ok());

        count
    }
}
