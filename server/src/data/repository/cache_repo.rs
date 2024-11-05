use std::collections::HashMap;

use async_trait::async_trait;
use common::{convert::{from_json_slice, to_json_vec}, data::dto::cache_config::CacheConfig};
use redis::{aio::MultiplexedConnection, AsyncCommands};

const REDIS_KEY_CACHE: &str = "request_cache"; // hash
const REDIS_KEY_CACHE_CONFIG: &str = "request_cache_config"; // hash

#[async_trait]
pub trait CacheRepo {
    async fn get(&self, key: String) -> Result<Vec<u8>, String>;
    async fn set(&self, key: String, data: Vec<u8>) -> Result<(), String>;
    async fn get_configs(&self) -> Result<Vec<CacheConfig>, String>;
    async fn set_config(&self, key: String, config: CacheConfig) -> Result<(), String>;
    async fn remove_config(&self, key: String) -> Result<(), String>;
}

pub struct CacheRepoImpl {
    connection: MultiplexedConnection
}

impl CacheRepoImpl {
    pub fn new(connection: MultiplexedConnection) -> Self {
        CacheRepoImpl { connection }
    }
}

#[async_trait]
impl CacheRepo for CacheRepoImpl {
    async fn get(&self, id: String) -> Result<Vec<u8>, String> {
        let data: Vec<u8> = self.connection.clone().hget(REDIS_KEY_CACHE, id.clone()).await
            .map_err(|e| format!("Error getting cache {}: {}", id, e))?;
        Ok(data)
    }

    async fn set(&self, key: String, data: Vec<u8>) -> Result<(), String> {
        self.connection.clone().hset(REDIS_KEY_CACHE, key.clone(), data).await
            .map_err(|e| format!("Error setting cache {}: {}", key, e))?;
        Ok(())
    }

    async fn get_configs(&self) -> Result<Vec<CacheConfig>, String> {
        let data: HashMap<String, Vec<u8>> = self.connection.clone().hgetall(REDIS_KEY_CACHE_CONFIG).await
            .map_err(|e| format!("Error getting cache configs: {}", e))?;
        let mut res: Vec<CacheConfig> = Vec::new();
        for (_, value) in data {
            res.push(from_json_slice(&value).unwrap());
        }

        Ok(res)
    }

    async fn set_config(&self, key: String, config: CacheConfig) -> Result<(), String> {
        self.connection.clone().hset(REDIS_KEY_CACHE_CONFIG, key.clone(), to_json_vec(&config)).await
            .map_err(|e| format!("Error setting cache config {}: {}", key, e))?;
        Ok(())
    }

    async fn remove_config(&self, key: String) -> Result<(), String> {
        self.connection.clone().hdel(REDIS_KEY_CACHE_CONFIG, key.clone()).await
            .map_err(|e| format!("Error unsetting cache config {}: {}", key, e))?;
        Ok(())
    }
}
