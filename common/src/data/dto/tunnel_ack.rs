use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelAck {
    pub id: String, // tunnel id
    pub success: bool,
    pub message: String,
    // public accessible endpoints
    // only server controls the access
    pub public_endpoints: Vec<String>,
}

impl TunnelAck {
    pub fn new(
        id: String,
        success: bool,
        message: String,
        public_endpoints: Vec<String>,
    ) -> Self {
        TunnelAck {
            id,
            success,
            message,
            public_endpoints,
        }
    }
}
