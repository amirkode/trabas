use chrono::NaiveDateTime;
use redis::{Client, Commands, RedisError};
use common::data::dto::tunnel_client::TunnelClient;

pub trait ClientRepo {
    fn get(&self, id: String) -> Result<TunnelClient, String>;
    fn create(&self, client: TunnelClient) -> Result<bool, String>;
    fn set_dc(&self, id: String, dt: NaiveDateTime) -> Result<bool, String>;
}

pub struct ClientRepoImpl {
    client: Client
}

impl ClientRepoImpl {
    pub fn new(client: Client) -> Self {
        ClientRepoImpl { client }
    }
}

impl ClientRepo for ClientRepoImpl {
    fn get(&self, id: String) -> Result<TunnelClient, String> {
        Err(String::from("implement this"))    
    }

    fn create(&self, client: TunnelClient) -> Result<bool, String> {
        Err(String::from("implement this"))
    }

    fn set_dc(&self, id: String, dt: NaiveDateTime) -> Result<bool, String> {
        Err(String::from("implement this"))
    }
}