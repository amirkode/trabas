
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PublicResponse {
    pub request_id: String,
    pub data: Vec<u8>
}