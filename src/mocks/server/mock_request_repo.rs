use std::{collections::VecDeque, sync::Arc};

use async_trait::async_trait;
use common::data::dto::public_request::PublicRequest;
use server::data::repository::request_repo::RequestRepo;
use tokio::sync::Mutex;


pub struct MockRequestRepo {
    mock_data: Arc<Mutex<VecDeque<PublicRequest>>>
}

impl MockRequestRepo {
    pub fn new() -> Self {
        MockRequestRepo {
            mock_data: Arc::new(Mutex::new(VecDeque::new()))
        }
    }
}

#[async_trait]
impl RequestRepo for MockRequestRepo {
    async fn push_back(&self, request: PublicRequest) -> Result<(), String> {
        self.mock_data.lock().await.push_back(request);
        Ok(())
    }

    async fn pop_front(&self, client_id: String) -> Result<PublicRequest, String> {
        if let Some(res) = self.mock_data.lock().await.pop_back() {
            return Ok(res)
        }
        Err(String::from("No request found"))
    }
}