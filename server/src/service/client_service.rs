use std::{sync::Arc};

use common::{data::dto::tunnel_client::TunnelClient};
use crate::data::repository::client_repo::ClientRepo;

#[derive(Clone)]
pub struct ClientService {
    client_repo: Arc<dyn ClientRepo + Send + Sync>
}

impl ClientService {
    pub fn new(client_repo: Arc<dyn ClientRepo + Send + Sync>) -> Self {
        ClientService { client_repo }
    }

    // register new client ID
    // if the new tunneling client attempts to connect
    // the client ID will be cached 
    pub async fn register_client(&self, client: TunnelClient, tunnel_id: String) -> Result<(), String> {
        let client_id = client.id.clone();
        let alias_id = client.alias_id.clone();
        // save to client information to redis store
        self.client_repo.create(client.clone(), tunnel_id).await?;
        // set alias
        self.client_repo.create_alias(alias_id, client_id.clone()).await?;
        Ok(())
    }

    pub async fn disconnect_client(&self, id: String, tunnel_id: String) -> Result<(), String> {
        let client = self.client_repo.get(id, tunnel_id.clone()).await?;
        // remove the alias
        self.client_repo.remove_alias(client.alias_id.clone()).await?;
        // remove the client
        self.client_repo.remove(client.id.clone(), tunnel_id).await
    }

    pub async fn check_client_validity(&self, id: String) -> Result<String, String> {
        let mut client_id = id.clone();
        let mut conn_cnt: i64 = match self.client_repo.get_connection_count(id.clone()).await {
            Ok(value) => value,
            Err(_) => 0
        };
        if conn_cnt < 1 {
            // check again in alias map
            let res: i64 = match self.client_repo.get_id_by_alias(id).await {
                Ok(id) => match self.client_repo.get_connection_count(id.clone()).await {
                    Ok(value) => {
                        client_id = id;
                        value
                    },
                    Err(_) => 0
                },
                Err(_) => 0
            };
            conn_cnt = res;
        }

        if conn_cnt > 0 {
            return Ok(client_id);
        }

        Err(String::from("Client invalid or inactive"))
    }

    pub async fn get_tunnel_count(&self, client_id: String) -> i64 {
        match self.client_repo.get_connection_count(client_id).await {
            Ok(count) => count,
            Err(_) => 0,
        }
    }
}
