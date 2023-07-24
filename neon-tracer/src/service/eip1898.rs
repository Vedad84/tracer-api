use {
    crate::{
        data_source::DataSource,
        metrics,
        service::Result,
        types::{geth::TransactionArgs, BlockNumber},
    },
    async_trait::async_trait,
    ethnum::U256,
    neon_cli_lib::types::Address,
    jsonrpsee::proc_macros::rpc,
    tracing::info,
    std::sync::atomic::Ordering,
};

#[rpc(server)]
#[async_trait]
pub trait EIP1898 {
    #[method(name = "eth_call")]
    async fn eth_call(&self, object: TransactionArgs, tag: BlockNumber) -> Result<String>;
    #[method(name = "eth_getStorageAt")]
    async fn eth_get_storage_at(
        &self,
        address: Address,
        index: U256,
        tag: BlockNumber,
    ) -> Result<U256>;
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
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        info!(
            "id {:?}: eth_call(caller={:?}, contract={:?}, gas={:?}, gasPrice={:?}, data={:?}, value={:?})",
            id,
            o.from,
            o.to,
            o.gas,
            o.gas_price,
            data.as_ref().map(hex::encode),
            o.value,
        );

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag, id).await?;
        let result = self
            .neon_api
            .emulate(o.from, o.to, o.value, data, slot, &tout, id)
            .await;
        info!("id {:?}: eth_call => {:?}", id, result);
        metrics::report_request_finished(started, "eth_call", result.is_ok());

        result
    }

    async fn eth_get_storage_at(
        &self,
        address: Address,
        index: U256,
        tag: BlockNumber,
    ) -> Result<U256> {
        let started = metrics::report_incoming_request("eth_getStorageAt");

        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        info!(
            "id {:?}: eth_getStorageAt({:?}, {:?}, {:?})",
            id, address, index, tag
        );

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag, id).await?;
        let value = self
            .neon_api
            .get_storage_at(address, index, slot, &tout, id)
            .await;
        info!("id {:?}: eth_getStorageAt => {:?}", id, value);
        metrics::report_request_finished(started, "eth_getStorageAt", value.is_ok());

        value
    }

    async fn eth_get_balance(&self, address: Address, tag: BlockNumber) -> Result<U256> {
        let started = metrics::report_incoming_request("eth_getBalance");

        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        info!("id {:?}: eth_getBalance({:?}, {:?})", id, address, tag);

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag, id).await?;
        let balance = self.neon_api.get_balance(address, slot, &tout, id).await;
        info!("id {:?}: eth_getBalance => {:?}", id, balance);
        metrics::report_request_finished(started, "eth_getBalance", balance.is_ok());

        balance
    }

    async fn eth_get_code(&self, address: Address, tag: BlockNumber) -> Result<String> {
        let started = metrics::report_incoming_request("eth_getCode");

        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        info!("id {:?}: eth_getCode({:?}, {:?})", id, address, tag);

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag, id).await?;
        let code = self.neon_api.get_code(address, slot, &tout, id).await;
        info!("id {:?}, eth_getCode => {:?}", id, code);
        metrics::report_request_finished(started, "eth_getCode", code.is_ok());

        code
    }

    async fn eth_get_transaction_count(&self, address: Address, tag: BlockNumber) -> Result<U256> {
        let started = metrics::report_incoming_request("eth_getTransactionCount");

        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        info!(
            "id {:?}: eth_getTransactionCount({:?}, {:?})",
            id, address, tag
        );

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag, id).await?;
        let count = self.neon_api.get_trx_count(address, slot, &tout, id).await;
        info!("id {:?}: eth_getTransactionCount => {:?}", id, count);
        metrics::report_request_finished(started, "eth_getTransactionCount", count.is_ok());

        count
    }
}
