use log::error;
use http::{Request, Response, StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::sync::Arc;
use tokio::sync::Mutex;
use common::data::dto::public_response::PublicResponse;
use crate::service::client_service::ClientService;
use crate::service::public_service::PublicService;

pub async fn register_public_handler(mut stream: TcpStream, service: PublicService) {
    tokio::spawn(async move {
        public_handler(stream, service).await;
    });
}

async fn public_handler(mut stream: TcpStream, service: PublicService) {
    // read data as bytes
    let mut raw_request = Vec::new();
    stream.read_to_end(&mut raw_request).await.unwrap();

    // parse the request
   

    // TODO: continue to enqueue request to redis
}

fn get_client_id() -> String {
    String::from("implement this")
}