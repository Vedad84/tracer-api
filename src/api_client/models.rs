use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct NeonApiResponse {
    pub result: String,
    pub value: String,
}
