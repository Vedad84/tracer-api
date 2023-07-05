use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct NeonApiResponse {
    pub result: String,
    pub value: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NeonApiError {
    pub result: String,
    pub error: Value,
}
