use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use common::{convert::{from_json_slice, to_json_vec}, data::dto::public_request::PublicRequest};

const REDIS_KEY_PUBLIC_REQUEST: &str = "public_requests";
const REDIS_KEY_PENDING_PUBLIC_REQUEST: &str = "pending_public_requests";

#[async_trait]
pub trait RequestRepo {
    async fn push_back(&self, request: PublicRequest) -> Result<(), String>;
    async fn pop_front(&self, client_id: String) -> Result<PublicRequest, String>;
    async fn queue_len(&self, client_id: String) -> Result<u16, String>;
    async fn ack_pending(&self, client_id: String, request_id: String) -> Result<(), String>;
    async fn ack_done(&self, client_id: String, request_id: String) -> Result<(), String>;
}

pub struct RequestRepoImpl {
    connection: MultiplexedConnection,
    
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

    async fn queue_len(&self, client_id: String) -> Result<u16, String> {
        let key = format!("{}_{}", REDIS_KEY_PENDING_PUBLIC_REQUEST, client_id);
        let queue_len: u16 = self.connection.clone().hlen(key.clone()).await
            .map_err(|e| format!("Error getting request queue len: {}", e))?;
        Ok(queue_len)
    }

    async fn ack_pending(&self, client_id: String, request_id: String) -> Result<(), String> {
        let flag = true;
        let data = to_json_vec(&flag);
        let key = format!("{}_{}", REDIS_KEY_PENDING_PUBLIC_REQUEST, client_id);
        self.connection.clone().hset(key, request_id.clone(), data).await
            .map_err(|e| format!("Error setting pending request {}: {}", request_id, e))?;
        Ok(())
    }

    async fn ack_done(&self, client_id: String, request_id: String) -> Result<(), String> {
        let key = format!("{}_{}", REDIS_KEY_PENDING_PUBLIC_REQUEST, client_id);
        self.connection.clone().hdel(key, request_id.clone()).await
            .map_err(|e| format!("Error unsetting pending request {}: {}", request_id, e))?;
        Ok(())
    }
}
