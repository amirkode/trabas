use std::{collections::VecDeque, sync::Arc};

use async_trait::async_trait;
use common::data::dto::public_request::PublicRequest;
use server::data::repository::request_repo::RequestRepo;
use tokio::sync::Mutex;


pub struct MockRequestRepo {
    mock_data: Arc<Mutex<VecDeque<PublicRequest>>>,
    request_limit: u16
}

impl MockRequestRepo {
    pub fn new(request_limit: u16) -> Self {
        MockRequestRepo {
            mock_data: Arc::new(Mutex::new(VecDeque::new())),
            request_limit
        }
    }
}

#[async_trait]
impl RequestRepo for MockRequestRepo {
    async fn push_back(&self, request: PublicRequest) -> Result<(), String> {
        if self.request_limit > 0 && self.mock_data.lock().await.len() as u16 > self.request_limit {
            return Err(String::from("Max request limit has been reached."))
        }

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