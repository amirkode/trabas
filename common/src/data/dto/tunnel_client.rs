use crate::security::{generate_hmac_key, sign_value};
use crate::version::validate_version;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelClient {
    pub id: String,
    // unique alias to the actual id, and generated every connection establishment
    pub alias_id: String,
    // signature value of id signed by client and verified by server
    pub signature: String,
    // client version code
    #[serde(default)]
    pub cl_version: String,
    // requested minimum server version code
    #[serde(default)]
    pub min_sv_version: String,
    // connection establied at
    pub conn_est_at: SystemTime,
    // connection disconnected at
    // # Deprecation Warning
    // This field is deprecated and might be removed in the future.
    #[serde(default)]
    pub conn_dc_at: Option<SystemTime>,
}

impl TunnelClient {
    pub fn new(id: String, signing_key: String, cl_version: String, min_sv_version: String) -> Self {
        let alias_id = generate_hmac_key(5); // as client nonce
        let mac = format!("{}_{}", id, alias_id);
        let signature = sign_value(mac, signing_key);
        TunnelClient {
            id,
            alias_id,
            signature,
            cl_version,
            min_sv_version,
            conn_est_at: SystemTime::now(),
            conn_dc_at: None,
        }
    }

    pub fn validate_version(&self, server_version: String, min_cl_server: String) -> bool {
        validate_version(server_version, self.min_sv_version.clone())
            && validate_version(self.cl_version.clone(), min_cl_server)
    }
}
