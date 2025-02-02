use std::collections::HashMap;

use async_trait::async_trait;
use common::{convert::{from_json_slice, to_json_vec}, data::dto::{cache::Cache, cache_config::CacheConfig}};
use redis::{aio::MultiplexedConnection, AsyncCommands};

const REDIS_KEY_CACHE: &str = "request_cache"; // hash
const REDIS_KEY_CACHE_CONFIG: &str = "request_cache_config"; // hash

#[async_trait]
pub trait CacheRepo {
    fn enabled(&self) -> bool;
    async fn get(&self, key: String) -> Result<Cache, String>;
    async fn set(&self, key: String, data: Cache) -> Result<(), String>;
    async fn get_configs(&self) -> Result<Vec<CacheConfig>, String>;
    async fn get_config(&self, key: String) -> Result<CacheConfig, String>;
    async fn set_config(&self, key: String, config: CacheConfig) -> Result<(), String>;
    async fn remove_config(&self, key: String) -> Result<(), String>;
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
    fn enabled(&self) -> bool {
        true
    }

    async fn get(&self, key: String) -> Result<Cache, String> {
        let data: Vec<u8> = self.connection.clone().hget(REDIS_KEY_CACHE, key.clone()).await
            .map_err(|e| format!("Error getting cache {}: {}", key, e))?;
        
        if data.len() == 0 {
            return Err(String::from("No cache was found"));
        }

        let res: Cache = from_json_slice(&data).unwrap();

        Ok(res)
    }

    async fn set(&self, key: String, cache: Cache) -> Result<(), String> {
        let data = to_json_vec(&cache);
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

    async fn get_config(&self, key: String) -> Result<CacheConfig, String> {
        let data: Vec<u8> = self.connection.clone().hget(REDIS_KEY_CACHE_CONFIG, key.clone()).await
            .map_err(|e| format!("Error getting cache {}: {}", key, e))?;
        
        if data.len() == 0 {
            return Err(String::from("No cache config was found"));
        }

        let res: CacheConfig = from_json_slice(&data).unwrap();

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

// cache with in process memory implementation
// TODO: implement this
//       might try to refactor redis config to be stored locally in file
pub struct CacheRepoProcMemImpl {
    // cache: Arc<Mutex<HashMap<String, Cache>>>,
    // cache_config: Arc<Mutex<HashMap<String, CacheConfig>>>
}

impl CacheRepoProcMemImpl {
    pub fn new() -> Self {
        CacheRepoProcMemImpl {
            // cache: Arc::new(Mutex::new(HashMap::new())),
            // cache_config: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl CacheRepo for CacheRepoProcMemImpl {
    fn enabled(&self) -> bool {
        // TODO: enable after implementation
        false
    }

    async fn get(&self, key: String) -> Result<Cache, String> {
        Err(String::from("Cache is only available for redis store."))
    }

    async fn set(&self, key: String, cache: Cache) -> Result<(), String> {
        Err(String::from("Cache is only available for redis store."))
    }

    async fn get_configs(&self) -> Result<Vec<CacheConfig>, String> {
        Err(String::from("Cache is only available for redis store."))
    }

    async fn get_config(&self, key: String) -> Result<CacheConfig, String> {
        Err(String::from("Cache is only available for redis store."))
    }

    async fn set_config(&self, key: String, config: CacheConfig) -> Result<(), String> {
        Err(String::from("Cache is only available for redis store."))
    }

    async fn remove_config(&self, key: String) -> Result<(), String> {
        Err(String::from("Cache is only available for redis store."))
    }
}
