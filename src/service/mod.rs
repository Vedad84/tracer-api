pub mod eip1898;
// pub mod geth;

pub type Error = jsonrpsee::types::error::ErrorObjectOwned;
pub type Result<T> = std::result::Result<T, Error>;
