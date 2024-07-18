use chrono::NaiveDateTime;
use redis::{Client, Commands, RedisError};
use common::data::dto::public_request::PublicRequest;

pub trait RequestRepo {
    fn push_back(&self, request: PublicRequest) -> Result<(), String>;
    fn pop_front(&self) -> Result<PublicRequest, String>;
}

pub struct RequestRepoImpl {
    client: Client
}

impl RequestRepoImpl {
    pub fn new(client: Client) -> Self {
        RequestRepoImpl { client }
    }
}

impl RequestRepo for RequestRepoImpl {
    fn push_back(&self, request: PublicRequest) -> Result<(), String> {
        Err(String::from("implement this"))
    }
    fn pop_front(&self) -> Result<PublicRequest, String> {
        Err(String::from("implement this"))
    }
}