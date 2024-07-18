use chrono::{Local, NaiveDateTime};

#[derive(Debug, Clone)]
pub struct TunnelClient {
    pub id: String,
    // connection establied at
    pub conn_est_at: NaiveDateTime,
    // connection disconnected at
    pub conn_dc_at: Option<NaiveDateTime> 
}

impl TunnelClient {
    pub fn new(id: String) -> Self {
        TunnelClient {
            id,
            conn_est_at: Local::now().naive_local(),
            conn_dc_at: None
        }
    }
}