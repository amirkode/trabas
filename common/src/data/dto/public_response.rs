
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct PublicResponse {
    pub request_id: String,
    // we don't want to send the tunnel_id from client service (upstream)
    // but, it's required after a package is received
    // by assigning the value from established tunnel
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tunnel_id: String,
    pub data: Vec<u8>
}

impl PublicResponse {
    pub fn new(request_id: String, tunnel_id: String, data: Vec<u8>) -> Self {
        PublicResponse { request_id, tunnel_id, data }
    }
}
