use {
    arrayref::array_ref,
    solana_sdk::pubkey::Pubkey,
    std::{fmt, sync::Arc,},
    tokio::task::block_in_place,
    web3::{ transports::Http, types::BlockId, Web3 },
    tracing::info,
    crate::{
        db::DbClient,
        evm_runtime::EVMRuntime,
        types::BlockNumber,
    },
    super::{Error, Result,  neon_cli::NeonCli},
    ethnum::U256,
};

fn convert_h256(inp: U256) -> web3::types::H256 {
    let a = inp.to_be_bytes();
    let bytes = array_ref![a, 0, 32];
    let hash = web3::types::H256::from(bytes);
    hash
}


#[derive(Clone)]
pub struct TracerCore {
    evm_loader: Pubkey,
    tracer_db_client: Arc<DbClient>,
    web3: Arc<Web3<Http>>,
    pub neon_cli: NeonCli,
}

impl std::fmt::Debug for TracerCore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            //"evm_loader={:?}, signer={:?}",
            "evm_loader={:?}",
            self.evm_loader //, self.signer
        )
    }
}


impl TracerCore {
    pub fn new(
        evm_loader: Pubkey,
        tracer_db_client: Arc<DbClient>,
        web3: Arc<Web3<Http>>,
        evm_runtime: Arc<EVMRuntime>,
    ) -> Self {

        Self {
            evm_loader,
            tracer_db_client,
            web3,
            neon_cli: NeonCli::new(evm_runtime),
        }
    }

    pub fn get_block_number(&self, tag: BlockNumber) -> Result<u64> {
        match tag {
            BlockNumber::Num(num) => Ok(num),
            BlockNumber::Hash { hash, .. } => {

                let hash_str = format!("0x{}", hex::encode(hash.to_be_bytes()));
                info!("Get block number {:?}", &hash_str);

                let future = self.web3
                    .eth()
                    .block(BlockId::Hash(convert_h256(hash)));

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
                self.tracer_db_client.get_earliest_slot().map_err(
                    |err| Error::Custom(format!("Failed to retrieve earliest block: {:?}", err))
                )
            },
            BlockNumber::Latest => {
                self.tracer_db_client.get_slot().map_err(
                    |err| Error::Custom(format!("Failed to retrieve latest block: {:?}", err))
                )
            },
            _ => {
                Err(Error::Custom("Unsupported block tag".to_string()))
            }
        }
    }
}
