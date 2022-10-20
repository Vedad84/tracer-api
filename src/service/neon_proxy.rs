use warp::header::value;
use {
    crate::{
        neon::{ Result, TracerCore },
        v1::{
            geth::types::trace::{ H256T },
            types::{ FilterTopic, FilterObject, LogObject }
        }
    },
    jsonrpsee::{ proc_macros::rpc, types::Error },
    tracing::{ instrument, info },
};

#[rpc(server)]
pub trait NeonProxy {
    #[method(name = "eth_getLogs")]
    fn eth_get_logs(
        &self,
        object: FilterObject,
    ) -> Result<Vec<LogObject>>;
}

impl NeonProxyServer for TracerCore {
    #[instrument]
    fn eth_get_logs(
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

        self.db_provider().get_logs(
            object.block_hash,
            from_block,
            to_block,
            Some(all_topics),
            object.address
        )
            .map_err(|err| Error::Custom(format!("Failed to read logs: {:?}", err)))
    }
}
