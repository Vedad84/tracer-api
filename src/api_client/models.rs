use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize)]
pub struct NeonApiResponse<T>
where T: fmt::Display
{
    pub result: String,
    pub value: T,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NeonApiError {
    pub result: String,
    pub error: String,
}

impl<T: fmt::Display> fmt::Display for NeonApiResponse<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(result: {}, value: {})",
            self.result,
            self.value
        )
    }
}
