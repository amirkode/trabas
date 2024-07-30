use async_trait::async_trait;
use client::data::repository::underlying_repo::UnderlyingRepo;
use common::net::{http_string_response_as_bytes, HttpResponse};
use http::StatusCode;

pub struct MockUnderlyingRepo {
    mock_response: String    
}

impl MockUnderlyingRepo {
    pub fn new(mock_response: String) -> Self {
        MockUnderlyingRepo {
            mock_response
        }
    }
}

#[async_trait]
impl UnderlyingRepo for MockUnderlyingRepo {
    async fn forward(&self, _: Vec<u8>, _: String) -> Result<Vec<u8>, String> {
        if let Ok(res) = http_string_response_as_bytes(self.mock_response.clone(), StatusCode::from_u16(400).unwrap()) {
            return Ok(res);
        }

        Err(String::from("An error occured"))
    }
}
