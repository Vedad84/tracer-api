mod neon_api;
pub mod tracer_db;

use {
    crate::{
        api_client::{client::Client as NeonAPIClient, config::Config as NeonAPIConfig},
        data_source::neon_api::NeonAPIDataSource,
        service::{Error, Result},
        types::BlockNumber,
    },
    arrayref::array_ref,
    jsonrpsee::types::error::ErrorCode,
    log::{info, warn},
    neon_cli_lib::types::{IndexerDb, TracerDb},
    std::sync::{atomic::AtomicU64, Arc},
    tracer_db::TracerDbExtention,
    web3::{transports::Http, types::BlockId, Web3},
};

pub const ERR: fn(&str, id: u64) -> Error = |e: &str, id: u64| -> Error {
    warn!("id {id:?}: error: {e}");
    let code = ErrorCode::InternalError;
    Error::owned(code.code(), code.message(), None::<()>)
};

#[derive(Clone)]
pub struct DataSource {
    tracer_db: TracerDb,
    pub indexer_db: IndexerDb,
    web3: Arc<Web3<Http>>,
    pub neon_api: NeonAPIDataSource,
    pub request_id: Arc<AtomicU64>,
}

impl DataSource {
    pub fn new(
        tracer_db: TracerDb,
        indexer_db: IndexerDb,
        web3: Arc<Web3<Http>>,
        neon_config: Arc<NeonAPIConfig>,
        neon_api_client: NeonAPIClient,
    ) -> Self {
        Self {
            tracer_db,
            indexer_db,
            web3,
            neon_api: NeonAPIDataSource::new(neon_config, neon_api_client),
            request_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub async fn get_block_number(&self, tag: BlockNumber, id: u64) -> Result<u64> {
        match tag {
            BlockNumber::Num(num) => Ok(num),
            BlockNumber::Hash { hash, .. } => {
                let hash = hash.to_be_bytes();

                let hash_str = format!("0x{}", hex::encode(hash));
                info!("id {:?}: Get block number for hash {:?}", id, &hash_str);

                let bytes = array_ref![hash, 0, 32];
                let hash_web3 = web3::types::H256::from(bytes);

                let block = self
                    .web3
                    .eth()
                    .block(BlockId::Hash(hash_web3))
                    .await
                    .map_err(|e| {
                        let e =
                            format!("id {id:?}: failed to send eth_getBlockByHash to proxy, {e:?}");
                        warn!("{e}");
                        Error::owned(ErrorCode::InternalError.code(), e, None::<()>)
                    })?
                    .ok_or_else(|| {
                        let e =
                            format!("id {id:?}: failed to obtain Block for BlockHash {hash_str:?}");
                        warn!("{e}");
                        Error::owned(ErrorCode::InternalError.code(), e, None::<()>)
                    })?;
                if let Some(blocknumber) = block.number {
                    info!("id {id:?}: BlockNumber: {blocknumber:?}");
                    Ok(blocknumber.as_u64())
                } else {
                    let e = format!("id {id:?}: BlockNumber is None for BlockHash {hash_str:?}");
                    warn!("{e}");
                    Err(Error::owned(ErrorCode::InternalError.code(), e, None::<()>))
                }
            }
            BlockNumber::Earliest => self.tracer_db.get_earliest_slot().map_err(|err| {
                Error::owned(
                    ErrorCode::InternalError.code(),
                    format!("Failed to retrieve earliest block: {err:?}"),
                    None::<()>,
                )
            }),
            BlockNumber::Latest => self.tracer_db.get_latest_block().await.map_err(|err| {
                Error::owned(
                    ErrorCode::InternalError.code(),
                    format!("Failed to retrieve latest block: {err:?}"),
                    None::<()>,
                )
            }),
            _ => {
                let e = format!("id {id:?}: Unsupported block tag {tag:?}");
                warn!("{e}");
                Err(Error::owned(ErrorCode::InternalError.code(), e, None::<()>))
            }
        }
    }
}

