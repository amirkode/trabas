use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use common::data::dto::public_response::PublicResponse;
use server::data::repository::response_repo::ResponseRepo;
use tokio::sync::Mutex;


pub struct MockResponseRepo {
    mock_data: Arc<Mutex<HashMap<String, PublicResponse>>>
}

impl MockResponseRepo {
    pub fn new() -> Self {
        MockResponseRepo {
            mock_data: Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

#[async_trait]
impl ResponseRepo for MockResponseRepo {
    async fn set(&self, response: PublicResponse) -> Result<(), String> {
        let key = response.clone().request_id;
        self.mock_data.lock().await.insert(key, response);
        Ok(())
    }

    async fn pop(&self, request_id: String) -> Result<PublicResponse, String> {
        if let Some(value) = self.mock_data.lock().await.get(&request_id) {
            return Ok((*value).clone());
        }
        Err(String::from("Data not found"))        
    }
}