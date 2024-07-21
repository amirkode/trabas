use std::time::SystemTime;

use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use common::{convert::{from_json_slice, to_json_vec}, data::dto::tunnel_client::TunnelClient};

const REDIS_KEY_CLIENT: &str = "tunnel_clients";

#[async_trait]
pub trait ClientRepo {
    async fn get(&self, id: String) -> Result<TunnelClient, String>;
    async fn create(&self, client: TunnelClient) -> Result<(), String>;
    async fn set_dc(&self, id: String, dt: SystemTime) -> Result<(), String>;
}

pub struct ClientRepoImpl {
    connection: MultiplexedConnection
}

impl ClientRepoImpl {
    pub fn new(connection: MultiplexedConnection) -> Self {
        ClientRepoImpl { connection }
    }
}

#[async_trait]
impl ClientRepo for ClientRepoImpl {
    async fn get(&self, id: String) -> Result<TunnelClient, String> {
        let data: Vec<u8> = self.connection.clone().hget(REDIS_KEY_CLIENT, id.clone()).await
            .map_err(|e| format!("Error getting client {}: {}", id, e))?;
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