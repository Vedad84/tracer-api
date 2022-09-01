pub mod eip1898;

use {
    crate::{ neon::TracerCore, v1::types::BlockNumber },
    jsonrpsee::types::error::Error,
};

type Result<T> = std::result::Result<T, Error>;
