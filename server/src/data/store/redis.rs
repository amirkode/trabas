use redis::{Client, RedisError};

pub struct RedisDataStore {
    client: redis::Client,
}

impl RedisDataStore {
    pub fn new(redis_url: &str) -> Result<Self, RedisError> {
        let client = Client::open(redis_url)?;
        Ok(RedisDataStore { client } )
    }
}