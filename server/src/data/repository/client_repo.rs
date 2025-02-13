use std::{collections::HashMap, sync::Arc, time::SystemTime};

use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use tokio::sync::Mutex;
use common::{convert::{from_json_slice, to_json_vec}, data::dto::tunnel_client::TunnelClient};

const REDIS_KEY_CLIENT_MAP: &str = "tunnel_clients";
const REDIS_KEY_CLIENT_ALIAS_MAP: &str = "tunnel_clients_alias_map";

#[async_trait]
pub trait ClientRepo {
    async fn get(&self, id: String) -> Result<TunnelClient, String>;
    async fn get_id_by_alias(&self, alias_id: String) -> Result<String, String>;
    async fn create(&self, client: TunnelClient) -> Result<(), String>;
    async fn remove_alias(&self, alias_id: String) -> Result<(), String>;
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
        let data: Vec<u8> = self.connection.clone().hget(REDIS_KEY_CLIENT_MAP, id.clone()).await
            .map_err(|e| format!("Error getting client {}: {}", id, e))?;
        if data.len() == 0 {
            return Err(String::from("Error getting client: no valid client exists"));
        }

        let res: TunnelClient = from_json_slice(&data).unwrap();
        Ok(res)
    }

    async fn get_id_by_alias(&self, alias_id: String) -> Result<String, String> {
        let data: String = self.connection.clone().hget(REDIS_KEY_CLIENT_ALIAS_MAP, alias_id.clone()).await
            .map_err(|e| format!("Error getting client ID by alias {}: {}", alias_id, e))?;
        if data.len() == 0 {
            return Err(String::from("Error getting client ID by alias: no valid client exists"));
        }

        Ok(data)
    }

    async fn create(&self, client: TunnelClient) -> Result<(), String> {
        let data = to_json_vec(&client);
        self.connection.clone().hset(REDIS_KEY_CLIENT_MAP, client.id.clone(), data).await
            .map_err(|e| format!("Error setting client {}: {}", client.id, e))?;
        // set alias map
        self.connection.clone().hset(REDIS_KEY_CLIENT_ALIAS_MAP, client.alias_id.clone(), client.id).await
            .map_err(|e| format!("Error setting client alias {}: {}", client.alias_id, e))?;
        Ok(())
    }

    async fn remove_alias(&self, alias_id: String) -> Result<(), String> {
        self.connection.clone().hdel(REDIS_KEY_CLIENT_ALIAS_MAP, alias_id.clone()).await
            .map_err(|e| format!("Error unsetting client alias {}: {}", alias_id, e))?;
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
    data: Arc<Mutex<HashMap<String, TunnelClient>>>,
    alias_map: Arc<Mutex<HashMap<String, String>>>
}

impl ClientRepoProcMemImpl {
    pub fn new() -> Self {
        ClientRepoProcMemImpl { 
            data: Arc::new(Mutex::new(HashMap::new())),
            alias_map: Arc::new(Mutex::new(HashMap::new())),
        }
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

    async fn get_id_by_alias(&self, alias_id: String) -> Result<String, String> {
        if let Some(value) = self.alias_map.lock().await.get(&alias_id) {
            return Ok((*value).clone());
        }
        Err(String::from("Error getting client ID by alias: no valid map exists"))
    }

    async fn create(&self, client: TunnelClient) -> Result<(), String> {
        let insert = client.clone();
        let key = client.id;
        let alias = client.alias_id;
        self.data.lock().await.insert(key.clone(), insert);
        // set alias map     
        self.alias_map.lock().await.insert(alias, key);
        Ok(())
    }

    async fn remove_alias(&self, alias_id: String) -> Result<(), String> {
        self.alias_map.lock().await.remove(&alias_id);
        Ok(())
    }

    async fn set_dc(&self, id: String, dt: SystemTime) -> Result<(), String> {
        let mut curr = self.get(id).await?;
        curr.conn_dc_at = Option::from(dt);
        self.create(curr).await
    }
}
