use {
    async_trait::async_trait,
    jsonrpsee::{ proc_macros::rpc, types::Error },
    log::*,
    crate::{
        metrics,
        neon::{tracer_core::TracerCore,  Result},
        types::{
            BlockNumber, geth::{TransactionArgs, TraceTransactionOptions, Trace, ExecutionResult},
        },
    },
};

#[rpc(server)]
#[async_trait]
pub trait GethTrace {
    #[method(name = "debug_traceCall")]
    async fn trace_call(&self, a: TransactionArgs, b: BlockNumber, o: Option<TraceTransactionOptions>) -> Result<Trace>;
}


#[async_trait]
impl GethTraceServer for TracerCore {
    async fn trace_call(&self, a: TransactionArgs,  tag: BlockNumber, o: Option<TraceTransactionOptions>) -> Result<Trace> {

        let data = a.input.map(|v| v.0);

        debug!(
            "geth::trace_call (from={:?}, to={:?}, data={:?}, value={:?}, gas={:?}, gasprice={:?})",
            a.from,
            a.to,
            data.as_ref().map(|v| hex::encode(&v)),
            a.value,
            a.gas,
            a.gas_price
        );

        let tout = std::time::Duration::new(10, 0);

        let started = metrics::report_incoming_request("geth::debug_traceCall");
        let slot = self.get_block_number(tag)?;
        let result = self.neon_cli.trace(a.from, a.to, a.value, data, a.gas, slot, &tout).await;
        metrics::report_request_finished(started, "geth::debug_traceCall", result.is_ok());

        result.map(|trace_call| {
            let o = o.unwrap_or_default();
            let response = Trace::Logs(ExecutionResult::new(trace_call,&o));
            debug!("response {:?}", response);
            response
        })
            .map_err(|e| {
                warn!("trace_call failed: {:?}", e);
                Error::Custom("Internal server error".to_string())
            })
    }
}