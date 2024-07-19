use std::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelClient {
    pub id: String,
    // signature value of id signed by client and verified by server
    pub signature: String,
    // connection establied at
    pub conn_est_at: SystemTime,
    // connection disconnected at
    pub conn_dc_at: Option<SystemTime> 
}

impl TunnelClient {
    pub fn new(id: String, signature: String) -> Self {
        TunnelClient {
            id,
            signature,
            conn_est_at: SystemTime::now(),
            conn_dc_at: None
        }
    }
}