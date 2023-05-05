use {
    async_trait::async_trait,
    jsonrpsee::proc_macros::rpc,
    log::*,
    neon_cli_lib::types::trace::{TraceCallConfig, TraceConfig},
    crate::{
        metrics,
        data_source::{DataSource, ERR},
        service::Result,
        types::{
            BlockNumber, geth::{TransactionArgs, Trace, ExecutionResult},
        },
    },
    ethnum::U256,
};

#[rpc(server)]
#[async_trait]
pub trait GethTrace {
    #[method(name = "debug_traceCall")]
    async fn trace_call(&self, a: TransactionArgs, b: BlockNumber, o: Option<TraceCallConfig>) -> Result<Trace>;
    #[method(name = "debug_traceTransaction")]
    async fn trace_transaction(&self, t: U256, o: Option<TraceConfig>) -> Result<Trace>;
    #[method(name = "debug_traceBlockByNumber")]
    async fn trace_block_by_number(&self, b: BlockNumber, o: Option<TraceConfig>) -> Result<Vec<Trace>>;
    #[method(name = "debug_traceBlockByHash")]
    async fn trace_block_by_hash(&self, bh: U256, o: Option<TraceConfig>) -> Result<Vec<Trace>>;
}


#[async_trait]
impl GethTraceServer for DataSource {
    async fn trace_call(&self, a: TransactionArgs, tag: BlockNumber, o: Option<TraceCallConfig>) -> Result<Trace> {
        let started = metrics::report_incoming_request("debug_traceCall");

        let data = a.input.map(|v| v.0);
        info!(
            "debug_traceCall(from={:?}, to={:?}, data={:?}, value={:?}, gas={:?}, gasprice={:?})",
            a.from,
            a.to,
            data.as_ref().map(|v| hex::encode(&v)),
            a.value,
            a.gas,
            a.gas_price
        );

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag)?;
        let result = self.neon_cli.trace(a.from, a.to, a.value, data, a.gas, slot, o.clone(), &tout).await;

        let result = result.map(|trace_call| {
            let o = o.unwrap_or_default();
            let response = Trace::Logs(ExecutionResult::new(trace_call,&o.trace_config));
            info!("debug_traceCall => {:?}", response);
            response
        });
        metrics::report_request_finished(started, "debug_traceCall", result.is_ok());

        result
    }

    async fn trace_transaction(&self, hash: U256, o: Option<TraceConfig>) -> Result<Trace> {
        let started = metrics::report_incoming_request("debug_traceTransaction");

        info!("debug_traceTransaction (hash={:?})", hash.to_string() );

        let tout = std::time::Duration::new(10, 0);
        let h = hash.to_be_bytes();
        let slot = self.indexer_db.get_slot(&h).map_err(|e | ERR(&format!("get_slot error: {}", e)))?;

        let result = self.neon_cli.trace_hash(hash, slot, o.clone(), &tout).await;

        let result = result.map(|trace_call| {
            let o = o.unwrap_or_default();
            let response = Trace::Logs(ExecutionResult::new(trace_call,&o));
            info!("debug_traceTransaction => {:?}", response);
            response
        });
        metrics::report_request_finished(started, "debug_traceTransaction", result.is_ok());

        result
    }

    async fn trace_block_by_number(&self, tag: BlockNumber, o: Option<TraceConfig>) -> Result<Vec<Trace>> {
        let started = metrics::report_incoming_request("debug_traceBlockByNumber");
        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag)?;
        let result = self.neon_cli.trace_block_by_slot(slot, o.clone(), &tout).await;

        let result = result.map(|trace_calls| {
            let o = o.unwrap_or_default();
            let response = trace_calls.into_iter()
                .map(|trace_call| Trace::Logs(ExecutionResult::new(trace_call,&o)))
                .collect();
            info!("debug_traceBlockByNumber => {:?}", response);
            response
        });
        metrics::report_request_finished(started, "debug_traceBlockByNumber", result.is_ok());

        result
    }

    async fn trace_block_by_hash(&self, bh: U256, o: Option<TraceConfig>) -> Result<Vec<Trace>> {
        let started = metrics::report_incoming_request("debug_traceBlockByHash");
        let tout = std::time::Duration::new(10, 0);
        let hash = bh.to_be_bytes();
        let slot = self.indexer_db.get_slot_by_block_hash(&hash)
            .map_err(|e | ERR(&format!("get_slot_by_block_hash error: {}", e)))?;
        let result = self.neon_cli.trace_block_by_slot(slot, o.clone(), &tout).await;

        let result = result.map(|trace_calls| {
            let o = o.unwrap_or_default();
            let response = trace_calls.into_iter()
                .map(|trace_call| Trace::Logs(ExecutionResult::new(trace_call,&o)))
                .collect();
            info!("debug_traceBlockByHash => {:?}", response);
            response
        });
        metrics::report_request_finished(started, "debug_traceBlockByHash", result.is_ok());

        result
    }
}

