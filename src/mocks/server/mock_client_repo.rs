use std::{collections::HashMap, sync::Arc, time::SystemTime};

use async_trait::async_trait;
use common::data::dto::tunnel_client::TunnelClient;
use server::data::repository::client_repo::ClientRepo;
use tokio::sync::Mutex;

pub struct MockClientRepo {
    mock_data: Arc<Mutex<HashMap<String, TunnelClient>>>
}

impl MockClientRepo {
    pub fn new() -> Self {
        MockClientRepo {
            mock_data: Arc::new(Mutex::new(HashMap::new()))
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

    async fn create(&self, client: TunnelClient) -> Result<(), String> {
        let key = client.clone().id;            
        self.mock_data.lock().await.insert(key, client);
        Ok(())
    }

    async fn set_dc(&self, id: String, dt: SystemTime) -> Result<(), String> {
        let mut curr = self.get(id).await?;
        curr.conn_dc_at = Option::from(dt);
        self.create(curr).await
    }
}