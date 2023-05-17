use {
    async_trait::async_trait,
    jsonrpsee::proc_macros::rpc,
    log::*,
    neon_cli_lib::types::trace::{TraceCallConfig, TraceConfig},
    crate::{
        data_source::{DataSource, ERR},
        metrics,
        service::Result,
        types::{
            geth::{ExecutionResult, Trace, TraceTransactionOptions, TransactionArgs},
            BlockNumber,
            BlockNumber, geth::{TransactionArgs, Trace, ExecutionResult},
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
        o: Option<TraceCallConfig>,
    ) -> Result<Trace>;
    #[method(name = "debug_traceTransaction")]
    async fn trace_transaction(
        &self,
        t: U256,
        o: Option<TraceConfig>,
    ) -> Result<Trace>;
    #[method(name = "debug_traceBlockByNumber")]
    async fn trace_block_by_number(
        &self,
        b: BlockNumber,
        o: Option<TraceConfig>,
    ) -> Result<Vec<Trace>>;
    #[method(name = "debug_traceBlockByHash")]
    async fn trace_block_by_hash(
        &self,
        bh: U256,
        o: Option<TraceConfig>,
    ) -> Result<Vec<Trace>>;
}

#[async_trait]
impl GethTraceServer for DataSource {
    async fn trace_call(
        &self,
        a: TransactionArgs,
        tag: BlockNumber,
        o: Option<TraceCallConfig>,
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
            a.gas_price,
            o,
        );

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag, id)?;
        let result = self
            .neon_api
            .trace(a.from, a.to, a.value, data, a.gas, slot, &tout, id)
            .await;

        let result = result.map(|trace_call| {
            let o = o.unwrap_or_default();
            let response = Trace::Logs(ExecutionResult::new(trace_call,&o.trace_config));
            info!("id {:?}: debug_traceCall => {:?}", id, response);
            response
        });
        metrics::report_request_finished(started, "debug_traceCall", result.is_ok());

        result
    }

    async fn trace_transaction(
        &self,
        hash: U256,
        o: Option<TraceConfig>,
    ) -> Result<Trace> {
        let started = metrics::report_incoming_request("debug_traceTransaction");

        let id = rand::random::<u16>();
        info!("id {id}: debug_traceTransaction (hash={hash})");

        let tout = std::time::Duration::new(10, 0);
        let h = hash.to_be_bytes();
        let slot = self
            .indexer_db
            .get_slot(&h)
            .map_err(|e| ERR(&format!("get_slot error: {e}"), id))?;

        let result = self.neon_api.trace_hash(hash, slot, &tout, o.clone(), id).await;

        let result = result.map(|trace_call| {
            let o = o.unwrap_or_default();
            let response = Trace::Logs(ExecutionResult::new(trace_call, &o));
            info!("id {:?}: debug_traceTransaction => {:?}", id, response);
            response
        });
        metrics::report_request_finished(started, "debug_traceTransaction", result.is_ok());

        result
    }

    async fn trace_block_by_number(&self, tag: BlockNumber, o: Option<TraceConfig>) -> Result<Vec<Trace>> {
        let started = metrics::report_incoming_request("debug_traceBlockByNumber");

        info!("debug_traceBlockByNumber (tag={tag:?}, config={o:?})");

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

    async fn trace_block_by_hash(&self, hash: U256, o: Option<TraceConfig>) -> Result<Vec<Trace>> {
        let started = metrics::report_incoming_request("debug_traceBlockByHash");

        info!("debug_traceBlockByHash (hash={hash}, config={o:?})");

        let tout = std::time::Duration::new(10, 0);
        let hash = hash.to_be_bytes();
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

