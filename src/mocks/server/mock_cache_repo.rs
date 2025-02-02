use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use common::data::dto::{cache::Cache, cache_config::CacheConfig};
use server::data::repository::cache_repo::CacheRepo;
use tokio::sync::Mutex;

pub struct MockCacheRepo {
    mock_cache: Arc<Mutex<HashMap<String, Cache>>>,
    mock_cache_config: Arc<Mutex<HashMap<String, CacheConfig>>>
}

impl MockCacheRepo {
    pub fn new() -> Self {
        MockCacheRepo {
            mock_cache: Arc::new(Mutex::new(HashMap::new())),
            mock_cache_config: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl CacheRepo for MockCacheRepo {
    
    fn enabled(&self) -> bool {
        true
    }
    
    async fn get(&self, key: String) -> Result<Cache, String> {
        if let Some(value) = self.mock_cache.lock().await.get(&key) {
            return Ok((*value).clone());
        }
        Err(String::from("Data not found"))
    }

    async fn set(&self, key: String, data: Cache) -> Result<(), String> {
        self.mock_cache.lock().await.insert(key, data);
        Ok(())
    }

    async fn get_configs(&self) -> Result<Vec<CacheConfig>, String> {
        let mut res: Vec<CacheConfig> = Vec::new();
        for (_, value) in self.mock_cache_config.lock().await.iter() {
            res.push(value.clone());
        }

        Ok(res)
    }

    async fn get_config(&self, key: String) -> Result<CacheConfig, String> {
        if let Some(value) = self.mock_cache_config.lock().await.get(&key) {
            return Ok((*value).clone());
        }
        Err(String::from("Data not found"))
    }

    async fn set_config(&self, key: String, config: CacheConfig) -> Result<(), String> {
        self.mock_cache_config.lock().await.insert(key, config);
        Ok(())
    }

    async fn remove_config(&self, key: String) -> Result<(), String> {
        self.mock_cache_config.lock().await.remove(&key);
        Ok(())
    }
    
}