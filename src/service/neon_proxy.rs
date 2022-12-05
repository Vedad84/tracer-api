use warp::header::value;
use {
    crate::{
        metrics,
        neon::{ Result, TracerCore },
        v1::{
            geth::types::trace::{ H256T },
            types::{ FilterTopic, FilterObject, LogObject }
        }
    },
    jsonrpsee::{ proc_macros::rpc, types::Error },
    tracing::{ instrument, info },
    log::warn,
};

#[rpc(server)]
pub trait NeonProxy {
    #[method(name = "eth_getLogs")]
    fn eth_get_logs(
        &self,
        object: FilterObject,
    ) -> Result<Vec<LogObject>>;
}

impl TracerCore {
    fn eth_get_logs_impl(
        &self,
        object: FilterObject
    ) -> Result<Vec<LogObject>> {
        info!("eth_getLogs: {:?}", object);

        let from_block = match object.from_block
            .map(|v| self.get_block_number(v)) {
            Some(Ok(from_block)) => Some(from_block),
            Some(Err(err)) => return Err(err),
            None => None,
        };

        let to_block = match object.to_block
            .map(|v| self.get_block_number(v)) {
            Some(Ok(to_block)) => Some(to_block),
            Some(Err(err)) => return Err(err),
            None => None,
        };

        let mut all_topics: Vec<H256T> = Vec::new();

        object.topics.map(
            |topics|
                topics.into_iter().for_each(
                    |topic| match topic {
                        FilterTopic::Single(topic) => all_topics.push(topic),
                        FilterTopic::Many(mut topics) => all_topics.append(&mut topics),
                    }));

        self.indexer_db_provider().get_logs(
            object.block_hash,
            from_block,
            to_block,
            Some(all_topics),
            object.address
        )
            .map_err(|err| Error::Custom(format!("Failed to read logs: {:?}", err)))
    }
}

impl NeonProxyServer for TracerCore {
    #[instrument]
    fn eth_get_logs(
        &self,
        object: FilterObject
    ) -> Result<Vec<LogObject>> {
        let started = metrics::report_incoming_request("eth_getLogs");
        let result = self.eth_get_logs_impl(object);
        metrics::report_request_finished(started, "eth_getLogs", result.is_ok());
        return if let Err(err) = result {
            warn!("eth_get_logs failed: {:?}", err);
            Err(Error::Custom("Internal server error".to_string()))
        } else {
            result
        }
    }
}
