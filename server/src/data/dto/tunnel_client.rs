use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct TunnelClient {
    pub id: String,
    pub path: String,
    pub last_attempt: NaiveDateTime,
}