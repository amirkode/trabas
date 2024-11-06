use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CacheConfig {
    pub client_id: String,
    pub method: String,
    pub path: String,
    pub exp_duration: u32
}

impl CacheConfig {
    pub fn new(client_id: String, method: String, path: String, exp_duration: u32) -> Self {
        CacheConfig { client_id, method, path, exp_duration }
    }
}