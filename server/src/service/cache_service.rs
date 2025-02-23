use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use cli_table::{format::Justify, Cell, Style, Table};
use sha2::{Digest, Sha256};

use common::data::dto::{cache::Cache, cache_config::CacheConfig};
use common::config::ConfigHandler;
use common::convert::{from_json_string, to_json_string};
use common::_info;
use crate::data::repository::cache_repo::CacheRepo;

// #[deprecated(since = "New implementation using local.env file", note = "Ignore this.")]
#[derive(Clone)]
pub struct CacheService {
    cache_repo: Arc<dyn CacheRepo + Send + Sync>,
    config_handler: Arc<dyn ConfigHandler + Send + Sync>,
    config_key: String,
}

impl CacheService {
    pub fn new(
        cache_repo: Arc<dyn CacheRepo + Send + Sync>,
        config_handler: Arc<dyn ConfigHandler + Send + Sync>,
        config_key: String,
    ) -> Self {
        Self { cache_repo, config_handler, config_key }
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
            Err(_) => _info!("A valid cache for {} was found", key),
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

    async fn get_cache_configs(&self) -> Vec<CacheConfig> {
        // fetch data from config .env
        let configs = self.config_handler.get_configs().await;
        if configs.contains_key(&self.config_key) {
            // parse vector from config
            let value = configs.get(&self.config_key).unwrap();
            let res: Option<Vec<CacheConfig>> = from_json_string(value);
            if let Some(val) = res {
                return val;
            }
        }

        Vec::new()
    }

    pub async fn get_cache_config(&self, client_id: String, method: String, path: String) -> Result<CacheConfig, String> {
        let key = self.get_cache_config_key(
            client_id.clone(),
            method.clone(),
            path.clone(),
        );
        
        let cache_configs = self.get_cache_configs().await;
        for config in cache_configs {
            let comp_key = self.get_cache_config_key(
                config.client_id.clone(),
                config.method.clone(),
                config.path.clone(),
            );
            if key == comp_key {
                return Ok(config);
            }
        }

        Err(String::from(format!("No cache config found for client_id: {}, method: {}, path: {}.", client_id, method, path)))
    }

    pub async fn set_cache_config(&self, config: CacheConfig) -> Result<(), String> {
        let key = self.get_cache_config_key(
            config.client_id.clone(),
            config.method.clone(),
            config.path.clone(),
        );
        let mut cache_configs = self.get_cache_configs().await;
        let mut exists = false;
        for check_config in cache_configs.iter_mut() {
            let comp_key = self.get_cache_config_key(
                check_config.client_id.clone(),
                check_config.method.clone(),
                check_config.path.clone()
            );
            if key == comp_key {
                // if exists just update the duration of expiration
                check_config.exp_duration = config.exp_duration;
                exists = true;
                break;
            }
        }
        // if the key does not exist, just append to existing configs
        if !exists {
            cache_configs.push(config);
        }

        // write the updated config
        let config_value = to_json_string(&cache_configs);
        self.config_handler.set_configs(HashMap::from([
            (self.config_key.clone(), config_value)
        ])).await;

        Ok(())
    }

    pub async fn remove_cache_config(&self, client_id: String, method: String, path: String) -> Result<(), String> {
        let key = self.get_cache_config_key(
            client_id,
            method,
            path
        );
        let mut cache_configs = self.get_cache_configs().await;
        // take the config with the given key
        cache_configs.retain(|config| 
            self.get_cache_config_key(
                config.client_id.clone(), 
                config.method.clone(), 
                config.path.clone()
            ) != key);

        // write the updated config
        let config_value = to_json_string(&cache_configs);
        self.config_handler.set_configs(HashMap::from([
            (self.config_key.clone(), config_value)
        ])).await;

        Ok(())
    }

    pub async fn show_cache_config(&self) -> Result<(), String> {
        let mut configs = self.get_cache_configs().await;
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
