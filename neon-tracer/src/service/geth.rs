use std::sync::atomic::Ordering;

use async_trait::async_trait;
use ethnum::U256;
use evm_loader::evm::tracing::event_listener::trace::{TraceCallConfig, TraceConfig};
use jsonrpsee::proc_macros::rpc;
use tracing::info;

use crate::{
    data_source::{DataSource, ERR},
    metrics,
    service::Result,
    types::{
        geth::{ExecutionResult, Trace, TransactionArgs},
        BlockNumber,
    },
};

#[rpc(server)]
pub trait GethTrace {
    #[method(name = "debug_traceCall")]
    async fn trace_call(
        &self,
        a: TransactionArgs,
        b: BlockNumber,
        o: Option<TraceCallConfig>,
    ) -> Result<Trace>;
    #[method(name = "debug_traceTransaction")]
    async fn trace_transaction(&self, t: U256, o: Option<TraceConfig>) -> Result<Trace>;
    #[method(name = "debug_traceBlockByNumber")]
    async fn trace_block_by_number(
        &self,
        b: BlockNumber,
        o: Option<TraceConfig>,
    ) -> Result<Vec<Trace>>;
    #[method(name = "debug_traceBlockByHash")]
    async fn trace_block_by_hash(&self, bh: U256, o: Option<TraceConfig>) -> Result<Vec<Trace>>;
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
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        info!(
            "id {:?}: debug_traceCall(from={:?}, to={:?}, data={:?}, value={:?}, gas={:?}, gasprice={:?}, config={:?})",
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
        let slot = self.get_block_number(tag, id).await?;
        let result = self
            .neon_api
            .trace(
                a.from,
                a.to,
                a.value,
                data,
                a.gas,
                slot,
                o.clone(),
                &tout,
                id,
            )
            .await;

        let result = result.map(|trace_call| {
            let o = o.unwrap_or_default();
            let response = Trace::Logs(ExecutionResult::new(trace_call, &o.trace_config));
            info!("id {:?}: debug_traceCall => {:?}", id, response);
            response
        });
        metrics::report_request_finished(started, "debug_traceCall", result.is_ok());

        result
    }

    async fn trace_transaction(&self, hash: U256, o: Option<TraceConfig>) -> Result<Trace> {
        let started = metrics::report_incoming_request("debug_traceTransaction");

        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
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
            .await
            .map_err(|e| ERR(&format!("get_slot error: {e}"), id))?;

        let result = self
            .neon_api
            .trace_hash(hash, slot, o.clone(), &tout, id)
            .await;

        let result = result.map(|trace_call| {
            let o = o.unwrap_or_default();
            let response = Trace::Logs(ExecutionResult::new(trace_call, &o));
            info!("id {:?}: debug_traceTransaction => {:?}", id, response);
            response
        });
        metrics::report_request_finished(started, "debug_traceTransaction", result.is_ok());

        result
    }

    async fn trace_block_by_number(
        &self,
        tag: BlockNumber,
        o: Option<TraceConfig>,
    ) -> Result<Vec<Trace>> {
        let started = metrics::report_incoming_request("debug_traceBlockByNumber");

        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        info!("id {id}: debug_traceBlockByNumber (tag={tag:?}, config={o:?})");

        let tout = std::time::Duration::new(10, 0);
        let slot = self.get_block_number(tag, id).await?;
        if slot == 0 {
            return Err(ERR("Genesis block is not traceable", id));
        }

        let result = self
            .neon_api
            .trace_next_block(slot - 1, o.clone(), &tout, id)
            .await;

        let result = result.map(|trace_calls| {
            let o = o.unwrap_or_default();
            let response = trace_calls
                .0
                .into_iter()
                .map(|trace_call| Trace::Logs(ExecutionResult::new(trace_call, &o)))
                .collect();
            info!("debug_traceBlockByNumber => {:?}", response);
            response
        });
        metrics::report_request_finished(started, "debug_traceBlockByNumber", result.is_ok());

        result
    }

    async fn trace_block_by_hash(&self, hash: U256, o: Option<TraceConfig>) -> Result<Vec<Trace>> {
        let started = metrics::report_incoming_request("debug_traceBlockByHash");

        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        info!("id {id}: debug_traceBlockByHash (hash={hash}, config={o:?})");

        let tout = std::time::Duration::new(10, 0);
        let hash = hash.to_be_bytes();
        let slot = self
            .indexer_db
            .get_slot_by_block_hash(&hash)
            .await
            .map_err(|e| ERR(&format!("get_slot_by_block_hash error: {e}"), id))?;
        if slot == 0 {
            return Err(ERR("Genesis block is not traceable", id));
        }
        let result = self
            .neon_api
            .trace_next_block(slot - 1, o.clone(), &tout, id)
            .await;

        let result = result.map(|trace_calls| {
            let o = o.unwrap_or_default();
            let response = trace_calls
                .0
                .into_iter()
                .map(|trace_call| Trace::Logs(ExecutionResult::new(trace_call, &o)))
                .collect();
            info!("debug_traceBlockByHash => {:?}", response);
            response
        });
        metrics::report_request_finished(started, "debug_traceBlockByHash", result.is_ok());

        result
    }
}
