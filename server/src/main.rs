mod data;
mod handler;
mod service;

use std::sync::Arc;

use data::repository::client_repo::ClientRepoImpl;
use data::repository::request_repo::RequestRepoImpl;
use data::repository::response_repo::ResponseRepoImpl;
use data::store::redis::RedisDataStore;
use handler::public_handler::register_public_handler;
use handler::tunnel_handler::register_tunnel_handler;
use service::client_service::ClientService;
use service::public_service::PublicService;

use tokio::net::TcpListener;
use env_logger::{Env, Builder, Target};
#[tokio::main]
async fn main() {
    Builder::from_env(Env::default().default_filter_or("info"))
        .target(Target::Stdout)
        .format_timestamp_millis()
        .init();

    // init instances
    let public_listener = TcpListener::bind("0.0.0.0:80").await.unwrap();
    let client_listener = TcpListener::bind("0.0.0.0:9999").await.unwrap();
    let redis_store = RedisDataStore::new().unwrap();
    let client_repo = ClientRepoImpl::new(redis_store.client.clone());
    let request_repo = RequestRepoImpl::new(redis_store.client.clone());
    let response_repo = ResponseRepoImpl::new(redis_store.client.clone());
    let client_service = ClientService::new(Arc::new(client_repo));
    let public_service = PublicService::new(Arc::new(request_repo), Arc::new(response_repo));

    loop {
        tokio::select! {
            Ok((socket, _)) = public_listener.accept() => {
                register_public_handler(socket, public_service.clone()).await;
            }
            Ok((socket, _)) = client_listener.accept() => {
                register_tunnel_handler(socket, client_service.clone(), public_service.clone()).await;
            }
        }
    }
}
