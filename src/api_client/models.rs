use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct NeonApiResponse<T>
where T: fmt::Debug
{
    pub result: String,
    pub value: T,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NeonApiError {
    pub result: String,
    pub error: Value,
}
