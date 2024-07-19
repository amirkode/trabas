use std::sync::Arc;

use chrono::Local;
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
    pub fn register_client(&self, client: TunnelClient) -> Result<(), String> {
        // save to client information to redis store
        self.client_repo.create(client)
    }

    pub fn disconnect_client(&self, id: String) -> Result<(), String> {
        let now = Local::now().naive_local();
        self.client_repo.set_dc(id.clone(), now)
    }
}
