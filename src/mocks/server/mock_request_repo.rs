use std::{collections::{HashMap, VecDeque}, sync::Arc};

use async_trait::async_trait;
use common::data::dto::public_request::PublicRequest;
use server::data::repository::request_repo::RequestRepo;
use tokio::sync::Mutex;


pub struct MockRequestRepo {
    mock_request_data: Arc<Mutex<HashMap<String, VecDeque<PublicRequest>>>>,
    mock_request_states: Arc<Mutex<HashMap<String, HashMap<String, bool>>>>,
}

impl MockRequestRepo {
    pub fn new() -> Self {
        MockRequestRepo {
            mock_request_data: Arc::new(Mutex::new(HashMap::new())),
            mock_request_states: Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

#[async_trait]
impl RequestRepo for MockRequestRepo {
    async fn push_back(&self, client_id: String, request: PublicRequest) -> Result<(), String> {
        self.mock_request_data.lock().await.entry(client_id)
            .or_insert_with(VecDeque::new)
            .push_back(request);
        
        Ok(())
    }

    async fn pop_front(&self, client_id: String) -> Result<PublicRequest, String> {
        if let Some(queue) = self.mock_request_data.lock().await.get_mut(&client_id) {
            if let Some(res) = queue.pop_front() {
                return Ok(res)
            }
        }

        Err(String::from("No request found"))
    }

    async fn queue_len(&self, client_id: String) -> Result<u16, String> {
        if let Some(mp) = self.mock_request_states.lock().await.get_mut(&client_id) {
            return Ok(mp.len() as u16)
        }

        Ok(0)
    }

    async fn ack_pending(&self, client_id: String, request_id: String) -> Result<(), String> {
        self.mock_request_states.lock().await.entry(client_id)
            .or_insert_with(HashMap::new)
            .insert(request_id, true);
        Ok(())
    }
    
    async fn ack_done(&self, client_id: String, request_id: String) -> Result<(), String> {
        if let Some(mp) = self.mock_request_states.lock().await.get_mut(&client_id) {
            mp.remove(&request_id);
        }
        
        Ok(())
    }

    async fn is_pending(&self, client_id: String, request_id: String) -> bool {
        if let Some(mp) = self.mock_request_states.lock().await.get_mut(&client_id) {
            if let Some(_) = mp.get(&request_id) {
                return true
            }
        }
        
        false
    }
}
