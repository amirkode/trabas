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
    // client version code
    #[serde(default)]
    pub cl_version_code: usize,
    // requested minimum server version code
    #[serde(default)]
    pub min_sv_version_code: usize,
    // connection establied at
    pub conn_est_at: SystemTime,
    // connection disconnected at
    pub conn_dc_at: Option<SystemTime> 
}

impl TunnelClient {
    pub fn new(id: String, signature: String, cl_version_code: usize, min_sv_version_code: usize) -> Self {
        TunnelClient {
            id,
            // generate alias using a random hex string
            alias_id: generate_hmac_key(5),
            signature,
            cl_version_code,
            min_sv_version_code,
            conn_est_at: SystemTime::now(),
            conn_dc_at: None
        }
    }

    pub fn validate_version(&self, server_version_code: usize, min_cl_server_code: usize) -> bool {
        server_version_code >= self.min_sv_version_code && 
        self.cl_version_code >= min_cl_server_code
    }
}
