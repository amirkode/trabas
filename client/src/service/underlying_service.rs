use std::sync::Arc;

use crate::data::repository::underlying_repo::UnderlyingRepo;


#[derive(Clone)]
pub struct UnderlyingService {
    repo: Arc<dyn UnderlyingRepo + Send + Sync>
}

impl UnderlyingService {
    pub fn new(repo: Arc<dyn UnderlyingRepo + Send + Sync>) -> Self {
        UnderlyingService { repo }
    }

    pub async fn foward_request(&self, request: Vec<u8>, host: String) -> Result<Vec<u8>, String> {
        self.repo.forward(request, host).await
    }
}

