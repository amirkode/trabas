use std::{sync::Arc, time::SystemTime};

use common::data::dto::tunnel_client::TunnelClient;
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
    pub async fn register_client(&self, client: TunnelClient) -> Result<(), String> {
        // remove previous alias if exists
        if let Ok(c) = self.client_repo.get(client.id.clone()).await {
            self.client_repo.remove_alias(c.alias_id).await?;
        }
        // set alias
        self.client_repo.create_alias(client.alias_id.clone(), client.id.clone()).await?;
        // save to client information to redis store
        self.client_repo.create(client).await
    }

    pub async fn disconnect_client(&self, id: String) -> Result<(), String> {
        let mut client = self.client_repo.get(id).await?;
        // set disconnection timestamp to now
        client.conn_dc_at = Option::from(SystemTime::now());
        // update entire data
        self.client_repo.create(client).await
    }

    pub async fn check_client_validity(&self, id: String) -> Result<String, String> {
        let rec: Option<TunnelClient> = match self.client_repo.get(id.clone()).await {
            Ok(value) => Some(value),
            Err(_) => {
                // check again in alias map
                let res: Option<TunnelClient> = match self.client_repo.get_id_by_alias(id).await {
                    Ok(id) => match self.client_repo.get(id).await {
                        Ok(value) => Some(value),
                        Err(_) => None
                    },
                    Err(_) => None
                };
                res
            }
        };

        if let Some(rec) = rec {
            if rec.conn_dc_at == None {
                return Ok(rec.id);
            }
        }

        Err(String::from("Client invalid or inactive"))
    }
}
