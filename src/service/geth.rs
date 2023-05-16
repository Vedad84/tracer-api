use {
    crate::{
        data_source::{DataSource, ERR},
        metrics,
        service::Result,
        types::{
            geth::{ExecutionResult, Trace, TraceTransactionOptions, TransactionArgs},
            BlockNumber,
        },
    },
    async_trait::async_trait,
    ethnum::U256,
    jsonrpsee::proc_macros::rpc,
    log::info,
};

#[rpc(server)]
#[async_trait]
pub trait GethTrace {
    #[method(name = "debug_traceCall")]
    async fn trace_call(
        &self,
        a: TransactionArgs,
        b: BlockNumber,
        o: Option<TraceTransactionOptions>,
    ) -> Result<Trace>;
    #[method(name = "debug_traceTransaction")]
    async fn trace_transaction(
        &self,
        t: U256,
        o: Option<TraceTransactionOptions>,
    ) -> Result<Option<Trace>>;
}

#[async_trait]
impl GethTraceServer for DataSource {
    async fn trace_call(
        &self,
        a: TransactionArgs,
        tag: BlockNumber,
        o: Option<TraceTransactionOptions>,
    ) -> Result<Trace> {
        let started = metrics::report_incoming_request("debug_traceCall");

        let data = a.input.map(|v| v.0);
        let id = rand::random::<u16>();
        info!(
            "id {:?}: debug_traceCall(from={:?}, to={:?}, data={:?}, value={:?}, gas={:?}, gasprice={:?})",
            id,
            a.from,
            a.to,
            data.as_ref().map(hex::encode),
            a.value,
            a.gas,
            a.gas_price
        );

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag, id).await?;
        let result = self
            .neon_api
            .trace(a.from, a.to, a.value, data, a.gas, slot, &tout, id)
            .await;

        let result = result.map(|trace_call| {
            let o = o.unwrap_or_default();
            let response = Trace::Logs(ExecutionResult::new(trace_call, &o));
            info!("id {:?}: debug_traceCall => {:?}", id, response);
            response
        });
        metrics::report_request_finished(started, "debug_traceCall", result.is_ok());

        result
    }

    async fn trace_transaction(
        &self,
        hash: U256,
        o: Option<TraceTransactionOptions>,
    ) -> Result<Option<Trace>> {
        let started = metrics::report_incoming_request("debug_traceTransaction");

        let id = rand::random::<u16>();
        info!(
            "id {:?}: debug_traceTransaction (hash={:?})",
            id,
            hash.to_string()
        );

        let tout = std::time::Duration::new(10, 0);
        let h = hash.to_be_bytes();
        let slot = self
            .indexer_db
            .get_slot(&h)
            .map_err(|e| ERR(&format!("get_slot error: {e}"), id))?;

        let result = self.neon_api.trace_hash(hash, slot, &tout, id).await;

        let result = result.map(|trace_call| {
            let o = o.unwrap_or_default();
            let response = Trace::Logs(ExecutionResult::new(trace_call, &o));
            info!("id {:?}: debug_traceTransaction => {:?}", id, response);
            Some(response)
        });
        metrics::report_request_finished(started, "debug_traceTransaction", result.is_ok());

        result
    }
}
