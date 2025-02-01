use log::info;
use redis::{Client, RedisError};

use crate::config;

pub struct RedisDataStore {
    pub client: redis::Client,
}

impl RedisDataStore {
    pub fn new() -> Result<Self, RedisError> {
        let host = std::env::var(config::CONFIG_KEY_SERVER_REDIS_HOST).unwrap_or_default();
        let port = std::env::var(config::CONFIG_KEY_SERVER_REDIS_PORT).unwrap_or_default();
        let pass = std::env::var(config::CONFIG_KEY_SERVER_REDIS_PASS).unwrap_or_default();
        let redis_url = format!("redis://:{}@{}:{}/0", pass, host, port);
        let client = Client::open(redis_url)?;

        Ok(RedisDataStore { client } )
    }
}