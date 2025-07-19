use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use common::data::dto::tunnel_client::TunnelClient;
use server::data::repository::client_repo::ClientRepo;
use tokio::sync::Mutex;

pub struct MockClientRepo {
    mock_data: Arc<Mutex<HashMap<String, HashMap<String, TunnelClient>>>>,
    mock_alias_map: Arc<Mutex<HashMap<String, String>>>,
}

impl MockClientRepo {
    pub fn new() -> Self {
        MockClientRepo {
            mock_data: Arc::new(Mutex::new(HashMap::new())),
            mock_alias_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ClientRepo for MockClientRepo {
    async fn get(&self, client_id: String, tunnel_id: String) -> Result<TunnelClient, String> {
        let data = self.mock_data.lock().await;
        if let Some(inner) = data.get(&client_id) {
            if let Some(client) = inner.get(&tunnel_id) {
                return Ok(client.clone());
            }
        }
        Err(String::from("Data not found"))
    }

    async fn get_all(&self, id: String) -> Result<Vec<TunnelClient>, String> {
        let data = self.mock_data.lock().await;
        if let Some(inner) = data.get(&id) {
            return Ok(inner.values().cloned().collect());
        }
        Err(String::from("Data not found"))
    }

    async fn get_connection_count(&self, id: String) -> Result<i64, String> {
        let data = self.mock_data.lock().await;
        if let Some(inner) = data.get(&id) {
            return Ok(inner.len() as i64);
        }
        Ok(0)
    }

    async fn remove(&self, client_id: String, tunnel_id: String) -> Result<(), String> {
        let mut data = self.mock_data.lock().await;
        if let Some(inner) = data.get_mut(&client_id) {
            inner.remove(&tunnel_id);
        }
        Ok(())
    }

    async fn get_id_by_alias(&self, alias_id: String) -> Result<String, String> {
        if let Some(value) = self.mock_alias_map.lock().await.get(&alias_id) {
            return Ok((*value).clone());
        }
        Err(String::from("Data not found"))
    }

    async fn create(&self, client: TunnelClient, tunnel_id: String) -> Result<(), String> {
        let mut data = self.mock_data.lock().await;
        let entry = data.entry(client.id.clone()).or_insert_with(HashMap::new);
        entry.insert(tunnel_id, client);
        Ok(())
    }

    async fn create_alias(&self, alias_id: String, client_id: String) -> Result<(), String> {
        self.mock_alias_map.lock().await.insert(alias_id, client_id);
        Ok(())
    }

    async fn remove_alias(&self, alias_id: String) -> Result<(), String> {
        self.mock_alias_map.lock().await.remove(&alias_id);
        Ok(())
    }
}
