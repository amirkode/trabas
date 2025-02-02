use std::{collections::{HashMap, VecDeque}, sync::Arc};

use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use tokio::sync::Mutex;
use common::{convert::{from_json_slice, to_json_vec}, data::dto::public_request::PublicRequest};

const REDIS_KEY_PUBLIC_REQUEST: &str = "public_requests";
const REDIS_KEY_PENDING_PUBLIC_REQUEST: &str = "pending_public_requests";

#[async_trait]
pub trait RequestRepo {
    async fn push_back(&self, client_id: String, request: PublicRequest) -> Result<(), String>;
    async fn pop_front(&self, client_id: String) -> Result<PublicRequest, String>;
    async fn queue_len(&self, client_id: String) -> Result<u16, String>;
    async fn ack_pending(&self, client_id: String, request_id: String) -> Result<(), String>;
    async fn ack_done(&self, client_id: String, request_id: String) -> Result<(), String>;
    async fn is_pending(&self, client_id: String, request_id: String) -> bool;
}

// Redis implementation
pub struct RequestRepoRedisImpl {
    connection: MultiplexedConnection,
}

impl RequestRepoRedisImpl {
    pub fn new(connection: MultiplexedConnection) -> Self {
        RequestRepoRedisImpl { connection }
    }
}

#[async_trait]
impl RequestRepo for RequestRepoRedisImpl {
    async fn push_back(&self, client_id: String, request: PublicRequest) -> Result<(), String> {
        let data = to_json_vec(&request);
        let key = format!("{}_{}", REDIS_KEY_PUBLIC_REQUEST, client_id);
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

    async fn is_pending(&self, client_id: String, request_id: String) -> bool {
        let key = format!("{}_{}", REDIS_KEY_PENDING_PUBLIC_REQUEST, client_id);
        let data: Vec<u8> = self.connection.clone().hget(key, request_id.clone()).await.unwrap_or_default();
        return data.len() > 0
    }
}

// In process memory implementation
pub struct RequestRepoProcMemImpl {
    request_data: Arc<Mutex<HashMap<String, VecDeque<PublicRequest>>>>,
    request_states: Arc<Mutex<HashMap<String, HashMap<String, bool>>>>,
}

impl RequestRepoProcMemImpl {
    pub fn new() -> Self {
        RequestRepoProcMemImpl { 
            request_data: Arc::new(Mutex::new(HashMap::new())),
            request_states: Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

#[async_trait]
impl RequestRepo for RequestRepoProcMemImpl {
    async fn push_back(&self, client_id: String, request: PublicRequest) -> Result<(), String> {
        self.request_data.lock().await.entry(client_id)
            .or_insert_with(VecDeque::new)
            .push_back(request);
        
        Ok(())
    }

    async fn pop_front(&self, client_id: String) -> Result<PublicRequest, String> {
        if let Some(queue) = self.request_data.lock().await.get_mut(&client_id) {
            if let Some(res) = queue.pop_front() {
                return Ok(res)
            }
        }

        Err(String::from("Error popping request: no pending request was found"))
    }

    async fn queue_len(&self, client_id: String) -> Result<u16, String> {
        if let Some(mp) = self.request_states.lock().await.get_mut(&client_id) {
            return Ok(mp.len() as u16)
        }

        Ok(0)
    }

    async fn ack_pending(&self, client_id: String, request_id: String) -> Result<(), String> {
        self.request_states.lock().await.entry(client_id)
            .or_insert_with(HashMap::new)
            .insert(request_id, true);
        Ok(())
    }

    async fn ack_done(&self, client_id: String, request_id: String) -> Result<(), String> {
        if let Some(mp) = self.request_states.lock().await.get_mut(&client_id) {
            mp.remove(&request_id);
        }
        
        Ok(())
    }

    async fn is_pending(&self, client_id: String, request_id: String) -> bool {
        if let Some(mp) = self.request_states.lock().await.get_mut(&client_id) {
            if let Some(_) = mp.get(&request_id) {
                return true
            }
        }
        
        false
    }
}
