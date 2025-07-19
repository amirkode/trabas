use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use tokio::sync::Mutex;
use common::{convert::{from_json_slice, to_json_vec}, data::dto::tunnel_client::TunnelClient};

const REDIS_KEY_CLIENT_PREFIX: &str = "tunnel_clients_";
const REDIS_KEY_CLIENT_ALIAS_MAP: &str = "tunnel_clients_alias_map";

#[async_trait]
pub trait ClientRepo {
    async fn get_connection_count(&self, id: String) -> Result<i64, String>;
    async fn get(&self, client_id: String, tunnel_id: String) -> Result<TunnelClient, String>;
    async fn get_all(&self, id: String) -> Result<Vec<TunnelClient>, String>;
    async fn get_id_by_alias(&self, alias_id: String) -> Result<String, String>;
    async fn create(&self, client: TunnelClient, tunnel_id: String) -> Result<(), String>;
    async fn create_alias(&self, alias_id: String, client_id: String) -> Result<(), String>;
    async fn remove_alias(&self, alias_id: String) -> Result<(), String>;
    async fn remove(&self, client_id: String, tunnel_id: String) -> Result<(), String>;
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
    async fn get(&self, client_id: String, tunnel_id: String) -> Result<TunnelClient, String> {
        let key = format!("{}{}", REDIS_KEY_CLIENT_PREFIX, client_id);
        let data: Vec<u8> = self.connection.clone().hget(key, tunnel_id.clone()).await
            .map_err(|e| format!("Error getting tunnel {} for client {}: {}", tunnel_id, client_id, e))?;
        if data.is_empty() {
            return Err(String::from("Error getting tunnel: no valid tunnel exists for this client"));
        }
        let client: TunnelClient = from_json_slice(&data)
            .ok_or_else(|| String::from("Deserialization error: could not parse TunnelClient"))?;
        Ok(client)
    }
    async fn get_connection_count(&self, id: String) -> Result<i64, String> {
        let key = format!("{}{}", REDIS_KEY_CLIENT_PREFIX, id);
        let map: HashMap<String, Vec<u8>> = self.connection.clone().hgetall(key).await
            .map_err(|e| format!("Error getting connection count for {}: {}", id, e))?;
        Ok(map.len() as i64)
    }

    async fn get_all(&self, id: String) -> Result<Vec<TunnelClient>, String> {
        let key = format!("{}{}", REDIS_KEY_CLIENT_PREFIX, id);
        let map: HashMap<String, Vec<u8>> = self.connection.clone().hgetall(key).await
            .map_err(|e| format!("Error getting client {}: {}", id, e))?;
        if map.is_empty() {
            return Err(String::from("Error getting client: no valid client exists"));
        }
        let mut result = Vec::new();
        for (_tunnel_id, value) in map {
            let client: TunnelClient = from_json_slice(&value).unwrap();
            result.push(client);
        }
        Ok(result)
    }

    async fn get_id_by_alias(&self, alias_id: String) -> Result<String, String> {
        let data: String = self.connection.clone().hget(REDIS_KEY_CLIENT_ALIAS_MAP, alias_id.clone()).await
            .map_err(|e| format!("Error getting client ID by alias {}: {}", alias_id, e))?;
        if data.is_empty() {
            return Err(String::from("Error getting client ID by alias: no valid client exists"));
        }
        Ok(data)
    }

    async fn create(&self, client: TunnelClient, tunnel_id: String) -> Result<(), String> {
        let key = format!("{}{}", REDIS_KEY_CLIENT_PREFIX, client.id);
        let data = to_json_vec(&client);
        self.connection.clone().hset::<_, _, _, i32>(key, tunnel_id, data).await
            .map_err(|e| format!("Error setting client {}: {}", client.id, e))?;
        Ok(())
    }

    async fn create_alias(&self, alias_id: String, client_id: String) -> Result<(), String> {
        self.connection.clone().hset::<_, _, _, i32>(REDIS_KEY_CLIENT_ALIAS_MAP, alias_id.clone(), client_id).await
            .map_err(|e| format!("Error setting client alias {}: {}", alias_id, e))?;
        Ok(())
    }

    async fn remove_alias(&self, alias_id: String) -> Result<(), String> {
        self.connection.clone().hdel::<_, _, i32>(REDIS_KEY_CLIENT_ALIAS_MAP, alias_id.clone()).await
            .map_err(|e| format!("Error unsetting client alias {}: {}", alias_id, e))?;
        Ok(())
    }

    async fn remove(&self, client_id: String, tunnel_id: String) -> Result<(), String> {
        let key = format!("{}{}", REDIS_KEY_CLIENT_PREFIX, client_id);
        self.connection.clone().hdel::<_, _, i32>(key, tunnel_id.clone()).await
            .map_err(|e| format!("Error removing tunnel {} for client {}: {}", tunnel_id, client_id, e))?;
        Ok(())
    }
}

// In process memory implementation
pub struct ClientRepoProcMemImpl {
    data: Arc<Mutex<HashMap<String, HashMap<String, TunnelClient>>>>,
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
    async fn get(&self, client_id: String, tunnel_id: String) -> Result<TunnelClient, String> {
        let data = self.data.lock().await;
        if let Some(inner) = data.get(&client_id) {
            if let Some(client) = inner.get(&tunnel_id) {
                return Ok(client.clone());
            }
        }
        Err(String::from("Error getting tunnel: no valid tunnel exists for this client"))
    }
    async fn get_connection_count(&self, id: String) -> Result<i64, String> {
        let data = self.data.lock().await;
        if let Some(inner) = data.get(&id) {
            return Ok(inner.len() as i64);
        }

        Ok(0)
    }

    async fn get_all(&self, id: String) -> Result<Vec<TunnelClient>, String> {
        let data = self.data.lock().await;
        if let Some(inner) = data.get(&id) {
            return Ok(inner.values().cloned().collect());
        }

        Err(String::from("Error getting client: no valid client exists"))
    }

    async fn get_id_by_alias(&self, alias_id: String) -> Result<String, String> {
        if let Some(value) = self.alias_map.lock().await.get(&alias_id) {
            return Ok((*value).clone());
        }

        Err(String::from("Error getting client ID by alias: no valid map exists"))
    }

    async fn create(&self, client: TunnelClient, tunnel_id: String) -> Result<(), String> {
        let mut data = self.data.lock().await;
        let entry = data.entry(client.id.clone()).or_insert_with(HashMap::new);
        entry.insert(tunnel_id, client);
        Ok(())
    }

    async fn create_alias(&self, alias_id: String, client_id: String) -> Result<(), String> {
        self.alias_map.lock().await.insert(alias_id, client_id);
        Ok(())
    }

    async fn remove_alias(&self, alias_id: String) -> Result<(), String> {
        self.alias_map.lock().await.remove(&alias_id);
        Ok(())
    }

    async fn remove(&self, client_id: String, tunnel_id: String) -> Result<(), String> {
        let mut data = self.data.lock().await;
        if let Some(inner) = data.get_mut(&client_id) {
            inner.remove(&tunnel_id);
        }
        Ok(())
    }
}
