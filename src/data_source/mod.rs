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
    tokio::task::block_in_place,
    std::sync::Arc,
    tracer_db::TracerDbExtention,
    web3::{transports::Http, types::BlockId, Web3},
};

pub const ERR: fn(&str, id: u16) -> Error = |e: &str, id: u16| -> Error {
    warn!("id {:?}: error: {}", id, e);
    Error::Custom("Internal server error".to_string())
};

#[derive(Clone)]
pub struct DataSource {
    tracer_db: TracerDb,
    pub indexer_db: IndexerDb,
    web3: Arc<Web3<Http>>,
    pub neon_api: NeonAPIDataSource,
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
        }
    }

    pub fn get_block_number(&self, tag: BlockNumber, id: u16) -> Result<u64> {
        match tag {
            BlockNumber::Num(num) => Ok(num),
            BlockNumber::Hash { hash, .. } => {

                let hash = hash.to_be_bytes();

                let hash_str = format!("0x{}", hex::encode(hash));
                info!("id {:?}: Get block number {:?}", id, &hash_str);

                let bytes = array_ref![hash, 0, 32];
                let hash_web3 = web3::types::H256::from(bytes);

                let future = self.web3
                    .eth()
                    .block(BlockId::Hash(hash_web3));

                let result = block_in_place(|| {
                    let handle = tokio::runtime::Handle::current();
                    handle.block_on(future)
                }).map_err(|err| Error::Custom(format!("Failed to get block number: {:?}", err)))?;

                Ok(result
                    .ok_or_else(|| Error::Custom(format!("Failed to obtain block number for hash: {}", hash_str)))?
                    .number
                    .ok_or_else(|| Error::Custom(format!("Failed to obtain block number for hash: {}", hash_str)))?
                    .as_u64())
            },
            BlockNumber::Earliest => {
                self.tracer_db.get_earliest_slot().map_err(
                    |err| Error::Custom(format!("Failed to retrieve earliest block: {:?}", err))
                )
            },
            BlockNumber::Latest => {
                self.tracer_db.get_latest_block().map_err(
                    |err| Error::Custom(format!("Failed to retrieve latest block: {:?}", err))
                )
            },
            _ => {
                Err(Error::Custom("Unsupported block tag".to_string()))
            }
        }
    }
}
