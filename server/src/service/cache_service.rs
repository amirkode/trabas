use cli_table::{format::Justify, Cell, Style, Table};
use common::data::dto::cache_config::CacheConfig;
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

    fn get_cache_key(&self, client_id: String, raw_request: Vec<u8>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(client_id.as_bytes());
        hasher.update(raw_request);

        let hash_result = hasher.finalize();
        format!("{:x}", hash_result)
    }

    pub async fn get_cache(
        &self,
        client_id: String,
        raw_request: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        let key = self.get_cache_key(client_id, raw_request);
        self.cache_repo.get(key).await
    }

    pub async fn set_cache(
        &self,
        client_id: String,
        raw_request: Vec<u8>,
        data: Vec<u8>,
    ) -> Result<(), String> {
        let key = self.get_cache_key(client_id, raw_request);
        self.cache_repo.set(key, data).await
    }

    fn get_cache_config_key(&self, client_id: String, path: String) -> String {
        let mut hasher = Sha256::new();
        hasher.update(client_id.as_bytes());
        hasher.update(path.as_bytes());

        let hash_result = hasher.finalize();
        format!("{:x}", hash_result)
    }

    pub async fn set_cache_config(&self, config: CacheConfig) -> Result<(), String> {
        let key = self.get_cache_config_key(
            config.client_id.clone(),
            config.path.clone(),
        );
        self.cache_repo.set_config(key, config).await
    }

    pub async fn remove_cache_config(&self, client_id: String, path: String) -> Result<(), String> {
        let key = self.get_cache_config_key(
            client_id,
            path
        );
        self.cache_repo.remove_config(key).await
    }

    pub async fn show_cache_config(&self) -> Result<(), String> {
        let configs = self.cache_repo.get_configs().await?;
        let table = configs
            .iter()
            .map(|config| {
                vec![
                    config.clone().client_id.cell().justify(Justify::Left),
                    config.clone().path.cell().justify(Justify::Left),
                    config.clone().exp_duration.cell().justify(Justify::Center),
                ]
            })
            .table()
            .title(vec![
                "Client ID".cell().bold(true),
                "Path".cell().bold(true),
                "Expiry Duration (Seconds)".cell().bold(true),
            ])
            .bold(true);

        let table_display = table.display().map_err(|e| format!("{}", e))?;

        println!("{}", table_display);
        
        Ok(())
    }
}
