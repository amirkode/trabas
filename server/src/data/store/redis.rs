use redis::{Client, RedisError};

pub struct RedisDataStore {
    pub client: redis::Client,
}

impl RedisDataStore {
    pub fn new() -> Result<Self, RedisError> {
        let redis_url = String::from("redis://127.0.0.1/");
        let client = Client::open(redis_url)?;
        Ok(RedisDataStore { client } )
    }
}