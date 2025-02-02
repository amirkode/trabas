use std::{collections::HashMap, sync::Arc, time::SystemTime};

use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use tokio::sync::Mutex;
use common::{convert::{from_json_slice, to_json_vec}, data::dto::tunnel_client::TunnelClient};

const REDIS_KEY_CLIENT: &str = "tunnel_clients";

#[async_trait]
pub trait ClientRepo {
    async fn get(&self, id: String) -> Result<TunnelClient, String>;
    async fn create(&self, client: TunnelClient) -> Result<(), String>;
    async fn set_dc(&self, id: String, dt: SystemTime) -> Result<(), String>;
}

// Redis implementation
pub struct ClientRepoRedisImpl {
    connection: MultiplexedConnection
}

impl ClientRepoRedisImpl {
    pub fn new(connection: MultiplexedConnection) -> Self {
        ClientRepoRedisImpl { connection }
    }
}

#[async_trait]
impl ClientRepo for ClientRepoRedisImpl {
    async fn get(&self, id: String) -> Result<TunnelClient, String> {
        let data: Vec<u8> = self.connection.clone().hget(REDIS_KEY_CLIENT, id.clone()).await
            .map_err(|e| format!("Error getting client {}: {}", id, e))?;
        if data.len() == 0 {
            return Err(String::from("Error getting client: no valid client exists"));
        }

        let res: TunnelClient = from_json_slice(&data).unwrap();
        Ok(res)
    }

    async fn create(&self, client: TunnelClient) -> Result<(), String> {
        let data = to_json_vec(&client);
        self.connection.clone().hset(REDIS_KEY_CLIENT, client.id.clone(), data).await
            .map_err(|e| format!("Error setting client {}: {}", client.id, e))?;
        Ok(())
    }

    async fn set_dc(&self, id: String, dt: SystemTime) -> Result<(), String> {
        let mut curr_data = self.get(id).await?;
        curr_data.conn_dc_at = Option::from(dt);
        self.create(curr_data).await
    }
}

// In process memory implementation
pub struct ClientRepoProcMemImpl {
    data: Arc<Mutex<HashMap<String, TunnelClient>>>
}

impl ClientRepoProcMemImpl {
    pub fn new() -> Self {
        ClientRepoProcMemImpl { data: Arc::new(Mutex::new(HashMap::new())) }
    }
}

#[async_trait]
impl ClientRepo for ClientRepoProcMemImpl {
    async fn get(&self, id: String) -> Result<TunnelClient, String> {
        if let Some(value) = self.data.lock().await.get(&id) {
            return Ok((*value).clone());
        }
        Err(String::from("Error getting client: no valid client exists"))
    }

    async fn create(&self, client: TunnelClient) -> Result<(), String> {
        let key = client.clone().id;            
        self.data.lock().await.insert(key, client);
        Ok(())
    }

    async fn set_dc(&self, id: String, dt: SystemTime) -> Result<(), String> {
        let mut curr = self.get(id).await?;
        curr.conn_dc_at = Option::from(dt);
        self.create(curr).await
    }
}
