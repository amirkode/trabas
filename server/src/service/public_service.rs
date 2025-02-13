use std::sync::Arc;
use tokio::time::{Instant, sleep, Duration};

use common::data::dto::public_request::PublicRequest;
use common::data::dto::public_response::PublicResponse;
use crate::data::repository::request_repo::RequestRepo;
use crate::data::repository::response_repo::ResponseRepo;

#[derive(Clone)]
pub struct PublicService {
    request_repo: Arc<dyn RequestRepo + Send + Sync>,
    response_repo: Arc<dyn ResponseRepo + Send + Sync>,
    request_limit: u16
}

impl PublicService {
    pub fn new(
        request_repo: Arc<dyn RequestRepo + Send + Sync>,
        response_repo: Arc<dyn ResponseRepo + Send + Sync>,
        request_limit: u16
    ) -> Self {
        PublicService { request_repo, response_repo, request_limit }
    }

    // enqueue a public client request to temporary database (redis)
    // the request will further be forwarded to target client service (provider)
    pub async fn enqueue_request(&self, client_id: String, request: PublicRequest) -> Result<(), String> {
        // if the request limit is set, the queue len must be checked
        if self.request_limit > 0 {
            let queue_len = self.request_repo.queue_len(client_id.clone()).await?;
            // TODO: instead of return right away, make it wait for a particular seconds (?)
            if queue_len > self.request_limit {
                return Err(String::from("Max request limit has been reached"))
            }
        }

        // set request as pending
        (*self.request_repo).ack_pending(client_id.clone(), request.id.clone()).await?;

        // enqueue request
        (*self.request_repo).push_back(client_id, request).await
    }

    // dequeue from request queue (FIFO)
    // reconsider the return type to directly return Vec<u8>
    // since it's the type returned by redis
    pub async fn dequeue_request(&self, client_id: String) -> Result<PublicRequest, String> {
        (*self.request_repo).pop_front(client_id).await
    }

    // assign response to hashes mapped by request_id
    // the response is ready to be returned
    pub async fn assign_response(&self, client_id: String, response: PublicResponse) -> Result<(), String> {
        if !(*self.request_repo).is_pending(client_id.clone(), response.request_id.clone()).await {
            return Err(String::from(format!("Error assigning response for request [{}]: Request invalid/expired", response.request_id.clone())))
        }

        (*self.response_repo).set(client_id, response).await
    }

    // TODO: implement queue cleaning mechanism
    // get response by corresponding request id
    // it will always check the response until it's found in the cache
    // when the timeout is reached, it breaks and returns a timeout error
    pub async fn get_response(&self, client_id: String, request_id: String, timeout_in_secs: u64) -> Result<PublicResponse, String> {
        let start_time = Instant::now();
        let mut elapsed: u64;
        // add initial break for 4 ms
        sleep(Duration::from_millis(4)).await;
        loop {
            // check data and return right away if it's found
            let res = (*self.response_repo).pop(client_id.clone(), request_id.clone()).await;
            if res.is_ok() {
                // set request as done
                (*self.request_repo).ack_done(client_id, request_id).await?;
                return Ok(res.unwrap())
            }

            // add break interval for 10 ms
            sleep(Duration::from_millis(10)).await;
            elapsed = start_time.elapsed().as_secs();
            if elapsed >= timeout_in_secs {
                break;
            }
        }

        // set request as done
        (*self.request_repo).ack_done(client_id, request_id.clone()).await?;

        Err(String::from(format!("Error getting request [{}]: Timeout reached after {} seconds", request_id, elapsed)))   
    }
}
