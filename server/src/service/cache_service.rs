use std::cmp::Ordering;
use std::time::{Duration, SystemTime};
use cli_table::{format::Justify, Cell, Style, Table};
use common::data::dto::{cache::Cache, cache_config::CacheConfig};
use log::info;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::data::repository::cache_repo::CacheRepo;

#[derive(Clone)]
pub struct CacheService {
    cache_repo: Arc<dyn CacheRepo + Send + Sync>,
}

impl CacheService {
    pub fn new(cache_repo: Arc<dyn CacheRepo + Send + Sync>) -> Self {
        CacheService { cache_repo }
    }

    fn get_cache_key(&self, client_id: String, uri: String, method: String, body: Vec<u8>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(client_id.as_bytes());
        hasher.update(uri.as_bytes());
        hasher.update(method.as_bytes());
        hasher.update(body);

        let hash_result = hasher.finalize();
        format!("{:x}", hash_result)
    }

    // Get request cache, if it's already expired, it will return error
    pub async fn get_cache(
        &self,
        client_id: String,
        uri: String,
        method: String,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        let key = self.get_cache_key(client_id, uri, method, body);
        let cache = self.cache_repo.get(key.clone()).await?;
        match cache.expired_at.elapsed() {
            Ok(duration) => return Err(format!("Cache {} has been expired for {} seconds", key, duration.as_secs())),
            Err(_) => info!("A valid cache for {} was found", key),
        }

        Ok(cache.data)
    }

    // Set request with proper expiration
    pub async fn set_cache(
        &self,
        client_id: String,
        uri: String,
        method: String,
        body: Vec<u8>,
        data: Vec<u8>,
        cache_config: CacheConfig,
    ) -> Result<(), String> {
        let key = self.get_cache_key(client_id.clone(), uri, method, body);
        let expired_at = SystemTime::now() + Duration::new(cache_config.exp_duration as u64, 0);
        let cache = Cache::new(expired_at, data);
        // write cache
        self.cache_repo.set(key, cache).await
    }

    fn get_cache_config_key(&self, client_id: String, method: String, path: String) -> String {
        let mut hasher = Sha256::new();
        hasher.update(client_id.as_bytes());
        hasher.update(method.as_bytes());
        hasher.update(path.as_bytes());

        let hash_result = hasher.finalize();
        format!("{:x}", hash_result)
    }

    pub async fn get_cache_config(&self, client_id: String, method: String, path: String) -> Result<CacheConfig, String> {
        let key = self.get_cache_config_key(
            client_id,
            method,
            path,
        );
        self.cache_repo.get_config(key).await
    }

    pub async fn set_cache_config(&self, config: CacheConfig) -> Result<(), String> {
        let key = self.get_cache_config_key(
            config.client_id.clone(),
            config.method.clone(),
            config.path.clone(),
        );
        self.cache_repo.set_config(key, config).await
    }

    pub async fn remove_cache_config(&self, client_id: String, method: String, path: String) -> Result<(), String> {
        let key = self.get_cache_config_key(
            client_id,
            method,
            path
        );
        self.cache_repo.remove_config(key).await
    }

    pub async fn show_cache_config(&self) -> Result<(), String> {
        let mut configs = self.cache_repo.get_configs().await?;
        // sort by client id and path
        configs.sort_by(|a, b| {
            if a.client_id < b.client_id {
                return Ordering::Less;
            }
            
            if a.client_id > b.client_id {
                return Ordering::Greater;
            }

            if a.path < b.path {
                return Ordering::Less;
            }

            if a.path > b.path {
                return Ordering::Greater;
            }

            if a.method < b.method {
                return Ordering::Less;
            }

            Ordering::Greater
        });

        let table = configs
            .iter()
            .map(|config| {
                vec![
                    config.clone().client_id.cell().justify(Justify::Left),
                    config.clone().method.cell().justify(Justify::Left),
                    config.clone().path.cell().justify(Justify::Left),
                    config.clone().exp_duration.cell().justify(Justify::Center),
                ]
            })
            .table()
            .title(vec![
                "Client ID".cell().bold(true),
                "Method".cell().bold(true),
                "Path".cell().bold(true),
                "Expiry Duration (Seconds)".cell().bold(true),
            ])
            .bold(true);

        let table_display = table.display().map_err(|e| format!("{}", e))?;

        println!("Request Cache Configurations:");
        println!("{}", table_display);
        
        Ok(())
    }
}
