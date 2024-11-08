use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use common::{convert::{from_json_slice, to_json_vec}, data::dto::public_response::PublicResponse};

const REDIS_KEY_PUBLIC_RESPONSE: &str = "public_responses";

#[async_trait]
pub trait ResponseRepo {
    async fn set(&self, client_id: String, response: PublicResponse) -> Result<(), String>;
    async fn pop(&self, client_id: String, request_id: String) -> Result<PublicResponse, String>;
}

pub struct ResponseRepoImpl {
    connection: MultiplexedConnection
}

impl ResponseRepoImpl {
    pub fn new(connection: MultiplexedConnection) -> Self {
        ResponseRepoImpl { connection }
    }
}

#[async_trait]
impl ResponseRepo for ResponseRepoImpl {
    async fn set(&self, client_id: String, response: PublicResponse) -> Result<(), String> {
        let key = format!("{}_{}", REDIS_KEY_PUBLIC_RESPONSE, client_id);
        let data = to_json_vec(&response);
        self.connection.clone().hset(key, response.request_id.clone(), data).await
            .map_err(|e| format!("Error setting response {}: {}", response.request_id, e))?;
        Ok(())
    }

    async fn pop(&self, client_id: String, request_id: String) -> Result<PublicResponse, String> {
        let key = format!("{}_{}", REDIS_KEY_PUBLIC_RESPONSE, client_id);
        let data: Vec<u8> = self.connection.clone().hget(key.clone(), request_id.clone()).await
            .map_err(|e| format!("Error getting response {}: {}", request_id, e))?;
        if data.len() == 0 {
            return Err(String::from("Error getting response: no response available"));
        }

        let res: PublicResponse = from_json_slice(&data).unwrap();
        // delete data
        self.connection.clone().hdel(key, request_id.clone()).await
            .map_err(|e| format!("Error deleting {}: {}", request_id, e))?;
        Ok(res)
    }
}
