use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Cache {
    pub expired_at: SystemTime,
    pub data: Vec<u8>
}

impl Cache {
    pub fn new(expired_at: SystemTime, data: Vec<u8>) -> Self {
        Cache { expired_at, data }
    }
}
