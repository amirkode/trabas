use std::sync::Arc;

use common::data::dto::public_request::PublicRequest;
use common::data::dto::public_response::PublicResponse;
use crate::data::repository::request_repo::RequestRepo;
use crate::data::repository::response_repo::ResponseRepo;

#[derive(Clone)]
pub struct PublicService {
    request_repo: Arc<dyn RequestRepo + Send + Sync>,
    response_repo: Arc<dyn ResponseRepo + Send + Sync> 
}

impl PublicService {
    pub fn new(request_repo: Arc<dyn RequestRepo + Send + Sync>,
        response_repo: Arc<dyn ResponseRepo + Send + Sync> ) -> Self {
        PublicService { request_repo, response_repo }
    }

    // enqueue a public client request to temporary database (redis)
    // the request will further be forwarded to target client service (provider)
    pub async fn enqueue_request(&self, request: PublicRequest) -> Result<(), String> {
        Err(String::from("impelement this"))
    }

    // dequeue from request queeue (FIFO)
    pub async fn dequeue_request(&self) -> Result<Vec<u8>, String> {
        Err(String::from("implement this"))
    }

    pub async fn assign_response(&self, response: PublicResponse) -> Result<(), String> {
        Err(String::from("implement this"))
    }

    pub async fn pop_front_response(&self) -> Result<Vec<u8>, String> {
        Err(String::from("implement this"))
    }
}