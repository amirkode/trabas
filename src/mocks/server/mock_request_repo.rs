use std::{collections::{HashMap, VecDeque}, sync::Arc};

use async_trait::async_trait;
use common::data::dto::public_request::PublicRequest;
use server::data::repository::request_repo::RequestRepo;
use tokio::sync::Mutex;


pub struct MockRequestRepo {
    mock_request_data: Arc<Mutex<VecDeque<PublicRequest>>>,
    mock_request_states: Arc<Mutex<HashMap<String, bool>>>,
}

impl MockRequestRepo {
    pub fn new() -> Self {
        MockRequestRepo {
            mock_request_data: Arc::new(Mutex::new(VecDeque::new())),
            mock_request_states: Arc::new(Mutex::new(HashMap::<String, bool>::new()))
        }
    }
}

#[async_trait]
impl RequestRepo for MockRequestRepo {
    async fn push_back(&self, request: PublicRequest) -> Result<(), String> {
        self.mock_request_data.lock().await.push_back(request);
        Ok(())
    }

    async fn pop_front(&self, _: String) -> Result<PublicRequest, String> {
        if let Some(res) = self.mock_request_data.lock().await.pop_back() {
            return Ok(res)
        }
        Err(String::from("No request found"))
    }

    async fn queue_len(&self, _: String) -> Result<u16, String> {
        Ok(self.mock_request_states.lock().await.len() as u16)
    }

    async fn ack_pending(&self, _: String, request_id: String) -> Result<(), String> {
        self.mock_request_states.lock().await.insert(request_id, true);
        Ok(())
    }
    
    async fn ack_done(&self, _: String, request_id: String) -> Result<(), String> {
        self.mock_request_states.lock().await.remove(&request_id);
        Ok(())
    }
}