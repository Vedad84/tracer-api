use crate::api_client::errors::NeonAPIClientError;

pub mod client;
pub mod config;
mod errors;
pub mod models;

pub(crate) type Result<T> = std::result::Result<T, NeonAPIClientError>;
