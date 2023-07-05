use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NeonAPIClientError {
    #[error("ReqwestError: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Error: {0}")]
    Std(#[from] Box<dyn std::error::Error>),

    #[error("JsonrpcError: {0}")]
    JsonrpcError(#[from] jsonrpsee::types::error::Error),

    #[error("ParseResponseError: {0}, response: {1}")]
    ParseResponseError(String, String),

    #[error("OtherResponseStatusError - status: {0}")]
    OtherResponseStatusError(reqwest::StatusCode),

    #[error("NeonApiError: {0}")]
    NeonApiError(String),
}
