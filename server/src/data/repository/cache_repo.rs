use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use common::{convert::{from_json_slice, to_json_vec}, data::dto::cache::Cache};
use redis::{aio::MultiplexedConnection, AsyncCommands};
use tokio::sync::Mutex;

const REDIS_KEY_CACHE: &str = "request_cache"; // hash

#[async_trait]
pub trait CacheRepo {
    async fn get(&self, key: String) -> Result<Cache, String>;
    async fn set(&self, key: String, data: Cache) -> Result<(), String>;
}

// Cache with redis implementation
pub struct CacheRepoRedisImpl {
    connection: MultiplexedConnection
}

impl CacheRepoRedisImpl {
    pub fn new(connection: MultiplexedConnection) -> Self {
        CacheRepoRedisImpl { connection }
    }
}

#[async_trait]
impl CacheRepo for CacheRepoRedisImpl {

    async fn get(&self, key: String) -> Result<Cache, String> {
        let data: Vec<u8> = self.connection.clone().hget(REDIS_KEY_CACHE, key.clone()).await
            .map_err(|e| format!("Error getting cache {}: {}.", key, e))?;
        
        if data.len() == 0 {
            return Err(String::from("No cache was found."));
        }

        let res: Cache = from_json_slice(&data).unwrap();

        Ok(res)
    }

    async fn set(&self, key: String, cache: Cache) -> Result<(), String> {
        let data = to_json_vec(&cache);
        self.connection.clone().hset::<_, _, _, ()>(REDIS_KEY_CACHE, key.clone(), data).await
            .map_err(|e| format!("Error setting cache {}: {}.", key, e))?;

        Ok(())
    }
}

// cache with in process memory implementation
pub struct CacheRepoProcMemImpl {
    cache: Arc<Mutex<HashMap<String, Cache>>>,
}

impl CacheRepoProcMemImpl {
    pub fn new() -> Self {
        CacheRepoProcMemImpl {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl CacheRepo for CacheRepoProcMemImpl {
    async fn get(&self, key: String) -> Result<Cache, String> {
        if let Some(value) = self.cache.lock().await.get(&key) {
            return Ok((*value).clone());
        }

        Err(String::from("No cache was found."))
    }

    async fn set(&self, key: String, data: Cache) -> Result<(), String> {
        self.cache.lock().await.insert(key, data);
        Ok(())
    }    
}
