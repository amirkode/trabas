use chrono::NaiveDateTime;
use redis::{Client, Commands, RedisError};
use common::data::dto::public_response::PublicResponse;

pub trait ResponseRepo {
    fn set(&self, response: PublicResponse) -> Result<(), String>;
    fn pop(&self, request_id: String) -> Result<PublicResponse, String>;
}

pub struct ResponseRepoImpl {
    client: Client
}

impl ResponseRepoImpl {
    pub fn new(client: Client) -> Self {
        ResponseRepoImpl { client }
    }
}

impl ResponseRepo for ResponseRepoImpl {
    fn set(&self, response: PublicResponse) -> Result<(), String> {
        Err(String::from("implement this"))
    }

    fn pop(&self, request_id: String) -> Result<PublicResponse, String> {
        Err(String::from("implement this"))
    }
}