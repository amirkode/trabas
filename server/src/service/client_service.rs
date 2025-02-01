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
        // save to client information to redis store
        self.client_repo.create(client).await
    }

    pub async fn disconnect_client(&self, id: String) -> Result<(), String> {
        let now = SystemTime::now();
        self.client_repo.set_dc(id.clone(), now).await
    }

    pub async fn check_client_validity(&self, id: String) -> bool {
        let rec = match self.client_repo.get(id).await {
            Ok(value) => value,
            Err(_) => return false
        };

        rec.conn_dc_at == None
    }
}
