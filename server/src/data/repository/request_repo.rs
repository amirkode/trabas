use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use common::{convert::{from_json_slice, to_json_vec}, data::dto::public_request::PublicRequest};

const REDIS_KEY_PUBLIC_REQUEST: &str = "public_requests";

#[async_trait]
pub trait RequestRepo {
    async fn push_back(&self, request: PublicRequest) -> Result<(), String>;
    async fn pop_front(&self, client_id: String) -> Result<PublicRequest, String>;
}

pub struct RequestRepoImpl {
    connection: MultiplexedConnection
}

impl RequestRepoImpl {
    pub fn new(connection: MultiplexedConnection) -> Self {
        RequestRepoImpl { connection }
    }
}

#[async_trait]
impl RequestRepo for RequestRepoImpl {
    async fn push_back(&self, request: PublicRequest) -> Result<(), String> {
        let data = to_json_vec(&request);
        let key = format!("{}_{}", REDIS_KEY_PUBLIC_REQUEST, request.client_id);
        self.connection.clone().lpush(key, &data).await
            .map_err(|e| format!("Error pushing request {}: {}", request.id, e))?;
        Ok(())
    }
    async fn pop_front(&self, client_id: String) -> Result<PublicRequest, String> {
        let key = format!("{}_{}", REDIS_KEY_PUBLIC_REQUEST, client_id);
        let data: Vec<u8> = self.connection.clone().rpop(key, None).await
            .map_err(|e| format!("Error popping request: {}", e))?;
        if data.len() == 0 {
            return Err(String::from("Error popping request: no pending request was found"));
        }
        
        let res: PublicRequest = from_json_slice(&data).unwrap();
        Ok(res)
    }
}
