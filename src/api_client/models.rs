use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct NeonApiResponse {
    pub result: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NeonApiError {
    pub result: String,
    pub error: String,
}
