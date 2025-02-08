use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use common::data::dto::cache::Cache;
use server::data::repository::cache_repo::CacheRepo;
use tokio::sync::Mutex;

pub struct MockCacheRepo {
    mock_cache: Arc<Mutex<HashMap<String, Cache>>>,
}

impl MockCacheRepo {
    pub fn new() -> Self {
        MockCacheRepo {
            mock_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl CacheRepo for MockCacheRepo {
    async fn get(&self, key: String) -> Result<Cache, String> {
        if let Some(value) = self.mock_cache.lock().await.get(&key) {
            return Ok((*value).clone());
        }

        Err(String::from("Data not found"))
    }

    async fn set(&self, key: String, data: Cache) -> Result<(), String> {
        self.mock_cache.lock().await.insert(key.clone(), data);
        
        Ok(())
    }
}