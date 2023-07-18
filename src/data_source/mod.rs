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
    log::{info, warn},
    neon_cli_lib::types::{IndexerDb, TracerDb},
    std::sync::{Arc, atomic::AtomicU64},
    tracer_db::TracerDbExtention,
    web3::{transports::Http, types::BlockId, Web3},
};

pub const ERR: fn(&str, id: u64) -> Error = |e: &str, id: u64| -> Error {
    warn!("id {:?}: error: {}", id, e);
    Error::Custom("Internal server error".to_string())
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

                let result = self.web3
                    .eth()
                    .block(BlockId::Hash(hash_web3)).await;

                let result = result.map_err(|e| {
                    warn!{"id {:?}: failed to send eth_getBlockByHash to proxy, {:?}", id, e};
                    Error::Custom(format!("failed to send eth_getBlockByHash to proxy: {:?}", e))
                })?;

                match result {
                    None => {
                        warn!("id {:?}: failed to obtain Block for BlockHash {:?}", id, &hash_str);
                        Err(Error::Custom(format!("failed to obtain Block for BlockHash: {}", &hash_str)))
                    }
                    Some(block) => {
                        if let Some(blocknumber) = block.number{
                            info!("id {:?}: BlockNumber: {:?}", id, blocknumber);
                            Ok(blocknumber.as_u64())
                        } else {
                            warn!("id {:?}: BlockNumber is None for BlockHash {:?}", id, &hash_str);
                            Err(Error::Custom(format!("BlockNumber is None for BlockHash: {}", &hash_str)))
                        }
                    }
                }
            },
            BlockNumber::Earliest => {
                self.tracer_db.get_earliest_slot().map_err(
                    |e| {
                        warn!("id {:?}: Failed to retrieve earliest block {:?}", id, e);
                        Error::Custom(format!("Failed to retrieve earliest block: {:?}", e))
                    }
                )
            },
            BlockNumber::Latest => {
                self.tracer_db.get_latest_block().map_err(
                    |e| {
                        warn!("id {:?}: Failed to retrieve latest block {:?}", id, e);
                        Error::Custom(format!("Failed to retrieve latest block: {:?}", e))
                    }
                )
            },
            _ => {
                warn!("id {:?}: Unsupported block tag {:?}", id, tag);
                Err(Error::Custom(format!("Unsupported block tag: {:?}", tag)))
            }
        }
    }
}
