use redis::{Client, RedisError};

use common::{_info, config::keys as config_keys};

pub struct RedisDataStore {
    pub client: redis::Client,
}

impl RedisDataStore {
    pub fn new() -> Result<Self, RedisError> {
        _info!("Redis: initializing...");
        let host = std::env::var(config_keys::CONFIG_KEY_SERVER_REDIS_HOST).unwrap_or_default();
        let port = std::env::var(config_keys::CONFIG_KEY_SERVER_REDIS_PORT).unwrap_or_default();
        let pass = std::env::var(config_keys::CONFIG_KEY_SERVER_REDIS_PASS).unwrap_or_default();
        let redis_url = format!("redis://:{}@{}:{}/0", pass, host, port);
        let client = Client::open(redis_url)?;
        _info!("Redis: connecting to {}:{}", host, port);

        Ok(RedisDataStore { client } )
    }
}