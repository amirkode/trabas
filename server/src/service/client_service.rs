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

    pub fn register_client(&self, id: String) -> Result<bool, String> {
        // save to client information to redis store
        let tunnel_client = TunnelClient::new(id.clone());
        self.client_repo.create(tunnel_client)?;
        Err(String::from("implement this"))
    }

    pub fn disconnect_client(&self, id: String) -> Result<bool, String> {
        let now = Local::now().naive_local();
        self.client_repo.set_dc(id.clone(), now)?;
        Err(String::from("implement this"))
    }
}
