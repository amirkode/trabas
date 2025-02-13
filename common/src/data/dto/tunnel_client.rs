use std::time::SystemTime;
use serde::{Deserialize, Serialize};

use crate::security::generate_hmac_key;

#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelClient {
    pub id: String,
    // unique alias to the actual id, and generated every connection establishment
    pub alias_id: String,
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
            // generate alias using a random hex string
            alias_id: generate_hmac_key(5),
            signature,
            conn_est_at: SystemTime::now(),
            conn_dc_at: None
        }
    }
}