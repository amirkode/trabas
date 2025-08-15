use serde::{Deserialize, Serialize};

use crate::security::{generate_hmac_key, sign_value};

#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelAck {
    pub id: String, // tunnel id, as server nonce
    pub signature: String, // server signature
    pub success: bool,
    pub message: String,
    // public accessible endpoints
    // only server controls the access
    pub public_endpoints: Vec<String>,
}

impl TunnelAck {
    pub fn success(
        id: String,
        client_mac: String,
        signing_key: String,
        public_endpoints: Vec<String>,
    ) -> Self {
        let mac = format!("{}_{}", id, client_mac);
        let signature = sign_value(mac, signing_key);
        TunnelAck {
            id,
            signature,
            success: true,
            message: "ok".into(),
            public_endpoints,
        }
    }

    pub fn fails(tunnel_id: String, message: String) -> Self {
        TunnelAck {
            id: tunnel_id,
            signature: String::new(),
            success: false,
            message,
            public_endpoints: Vec::new(),
        }
    }
}
