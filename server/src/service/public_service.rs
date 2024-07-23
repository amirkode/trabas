use std::sync::Arc;
use tokio::time::{Instant, sleep, Duration};

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
        (*self.request_repo).push_back(request).await
    }

    // dequeue from request queue (FIFO)
    // reconsider the return type to directly return Vec<u8>
    // since it's the type returned by redis
    pub async fn dequeue_request(&self) -> Result<PublicRequest, String> {
        (*self.request_repo).pop_front().await
    }

    // assign response to hashes mapped by request_id
    // the response is ready to be returned
    pub async fn assign_response(&self, response: PublicResponse) -> Result<(), String> {
        (*self.response_repo).set(response).await
    }

    // get response by corresponding request id
    // it will always check the response until it's found in the cache
    // when the timeout is reached, it breaks and returns a timeout error
    pub async fn get_response(&self, request_id: String, timeout_in_secs: u64) -> Result<PublicResponse, String> {
        let start_time = Instant::now();
        let mut elapsed: u64;
        // add initial break for 4 ms
        sleep(Duration::from_millis(4)).await;
        loop {
            // check data and return right away if it's found
            let res = (*self.response_repo).pop(request_id.clone()).await;
            if res.is_ok() {
                return Ok(res.unwrap())
            }

            // add break interval for 10 ms
            sleep(Duration::from_millis(10)).await;
            elapsed = start_time.elapsed().as_secs();
            if elapsed >= timeout_in_secs {
                break;
            }
        }

        Err(String::from(format!("Error getting request [{}]: Timeout reached after {} seconds", request_id, elapsed)))   
    }
}