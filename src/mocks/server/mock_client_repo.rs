use std::{collections::HashMap, sync::Arc, time::SystemTime};

use async_trait::async_trait;
use common::data::dto::tunnel_client::TunnelClient;
use server::data::repository::client_repo::ClientRepo;
use tokio::sync::Mutex;

pub struct MockClientRepo {
    mock_data: Arc<Mutex<HashMap<String, TunnelClient>>>,
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
    async fn get(&self, id: String) -> Result<TunnelClient, String> {
        if let Some(value) = self.mock_data.lock().await.get(&id) {
            return Ok((*value).clone());
        }
        Err(String::from("Data not found"))
    }

    async fn get_id_by_alias(&self, alias_id: String) -> Result<String, String> {
        if let Some(value) = self.mock_alias_map.lock().await.get(&alias_id) {
            return Ok((*value).clone());
        }
        Err(String::from("Data not found"))
    }

    async fn create(&self, client: TunnelClient) -> Result<(), String> {
        let insert = client.clone();
        let key = client.id;
        self.mock_data.lock().await.insert(key.clone(), insert);
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