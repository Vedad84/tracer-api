pub mod eip1898;

use jsonrpsee::types::error::Error;

type Result<T> = std::result::Result<T, Error>;
