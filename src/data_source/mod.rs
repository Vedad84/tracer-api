mod neon_cli;
pub mod tracer_db;

use {
    std::sync::Arc,
    web3::{ transports::Http, Web3, types::BlockId, },
    crate::{
        evm_runtime::EVMRuntime,
        data_source::neon_cli::NeonCli,
        service::{Error, Result},
        types::BlockNumber,
    },
    neon_cli_lib::types::{TracerDb, IndexerDb},
    tokio::task::block_in_place,
    tracer_db::TracerDbExtention,
    log::{info, debug, warn},
    arrayref::array_ref,
};

pub const ERR: fn(&str)->Error = |e: &str| -> Error {
    warn!("error: {}", e);
    Error::Custom("Internal server error".to_string())
};

#[derive(Clone)]
pub struct DataSource {
    tracer_db: TracerDb,
    pub indexer_db: IndexerDb,
    web3: Arc<Web3<Http>>,
    pub neon_cli: NeonCli,
}

impl DataSource {
    pub fn new(
        tracer_db: TracerDb,
        indexer_db: IndexerDb,
        web3: Arc<Web3<Http>>,
        evm_runtime: Arc<EVMRuntime>,
    ) -> Self {
        Self {tracer_db, indexer_db,  web3, neon_cli: NeonCli::new(evm_runtime) }
    }

    pub fn get_block_number(&self, tag: BlockNumber) -> Result<u64> {
        match tag {
            BlockNumber::Num(num) => Ok(num),
            BlockNumber::Hash { hash, .. } => {

                let hash = hash.to_be_bytes();

                let hash_str = format!("0x{}", hex::encode(hash));
                debug!("Get block number {:?}", &hash_str);

                let bytes = array_ref![hash, 0, 32];
                let hash_web3 = web3::types::H256::from(bytes);

                let future = self.web3
                    .eth()
                    .block(BlockId::Hash(hash_web3));

                let result = block_in_place(|| {
                    let handle = tokio::runtime::Handle::current();
                    handle.block_on(future)
                }).map_err(|err| Error::Custom(format!("Failed to get block number: {:?}", err)))?;

                info!("Web3 part ready");

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
