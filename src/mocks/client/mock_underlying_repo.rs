use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use client::data::repository::underlying_repo::UnderlyingRepo;
use common::net::http_string_response_as_bytes;
use http::StatusCode;

pub struct MockUnderlyingRepo<F> 
where
    F: FnMut() -> ()
{
    mock_response: String,
    callback: Arc<Mutex<F>>
}

impl<F> MockUnderlyingRepo<F> 
where
    F: FnMut() -> ()
{
    pub fn new(mock_response: String, callback: Arc<Mutex<F>>) -> Self {
        MockUnderlyingRepo {
            mock_response,
            callback
        }
    }
}

#[async_trait]
impl<F> UnderlyingRepo for MockUnderlyingRepo<F>
where
    F: FnMut() -> () + Send + Sync + 'static
{
    async fn forward(&self, _: Vec<u8>, _: String) -> Result<Vec<u8>, String> {
        if let Ok(res) = http_string_response_as_bytes(self.mock_response.clone(), StatusCode::from_u16(200).unwrap()) {
            let mut callback = self.callback.lock().unwrap();
            callback();
            return Ok(res);
        }

        Err(String::from("An error occured"))
    }
}
