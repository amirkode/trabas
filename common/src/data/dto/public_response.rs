
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct PublicResponse {
    pub request_id: String,
    pub data: Vec<u8>
}

impl PublicResponse {
    pub fn new(request_id: String, data: Vec<u8>) -> Self {
        PublicResponse { request_id, data }
    }
}