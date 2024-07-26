pub mod data;
pub mod handler;
pub mod service;
pub mod types;
pub mod config;

use std::sync::Arc;
use log::info;

use config::validate_configs;
use data::repository::client_repo::ClientRepoImpl;
use data::repository::request_repo::RequestRepoImpl;
use data::repository::response_repo::ResponseRepoImpl;
use data::store::redis::RedisDataStore;
use handler::public_handler::register_public_handler;
use handler::tunnel_handler::register_tunnel_handler;
use service::client_service::ClientService;
use service::public_service::PublicService;

use tokio::net::TcpListener;

// entry point of the server service
pub async fn run(root_host: String, public_port: u16, client_port: u16) {
    validate_configs();
    // init instances
    let public_listener = TcpListener::bind(format!("{}:{}", root_host, public_port)).await.unwrap();
    let client_listener = TcpListener::bind(format!("{}:{}", root_host, client_port)).await.unwrap();
    let redis_store = RedisDataStore::new().unwrap();
    let redis_connection = redis_store.client.get_multiplexed_async_connection().await.unwrap();
    let client_repo = ClientRepoImpl::new(redis_connection.clone());
    let request_repo = RequestRepoImpl::new(redis_connection.clone());
    let response_repo = ResponseRepoImpl::new(redis_connection.clone());
    let client_service = ClientService::new(Arc::new(client_repo));
    let public_service = PublicService::new(Arc::new(request_repo), Arc::new(response_repo));

    info!("[Public Listerner] Listening on :{}", public_listener.local_addr().unwrap());
    info!("[Client Listerner] Listening on :{}", client_listener.local_addr().unwrap());

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
