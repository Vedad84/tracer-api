pub mod tracer_core;
pub mod neon_cli;

type Error = jsonrpsee::types::error::Error;

pub type Result<T> = std::result::Result<T, Error>;

