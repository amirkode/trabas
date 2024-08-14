pub mod data;
pub mod handler;
pub mod service;
pub mod types;
pub mod config;

use std::sync::Arc;
use log::info;

use config::validate_configs;
use data::repository::client_repo::{ClientRepo, ClientRepoImpl};
use data::repository::request_repo::{RequestRepo, RequestRepoImpl};
use data::repository::response_repo::{ResponseRepo, ResponseRepoImpl};
use data::store::redis::RedisDataStore;
use handler::public_handler::register_public_handler;
use handler::tunnel_handler::register_tunnel_handler;
use service::client_service::ClientService;
use service::public_service::PublicService;

use tokio::net::TcpListener;

// entry point of the server service
pub async fn entry_point(
    root_host: String, 
    public_port: u16, 
    client_port: u16,
    client_request_limit: u16,
) {
    validate_configs();
    let public_svc_address = format!("{}:{}", root_host, public_port);
    let client_svc_address = format!("{}:{}", root_host, client_port);
    // init repo to be injected
    let redis_store = RedisDataStore::new().unwrap();
    let redis_connection = redis_store.client.get_multiplexed_async_connection().await.unwrap();
    let client_repo = Arc::new(ClientRepoImpl::new(redis_connection.clone()));
    let request_repo = Arc::new(RequestRepoImpl::new(redis_connection.clone(), client_request_limit));
    let response_repo = Arc::new(ResponseRepoImpl::new(redis_connection.clone()));
    // run the services
    run(
        public_svc_address,
        client_svc_address,
        client_repo,
        request_repo,
        response_repo
    ).await;
}

pub async fn run(
    public_svc_address: String,
    client_svc_address: String,
    client_repo: Arc<dyn ClientRepo + Send + Sync>,
    request_repo: Arc<dyn RequestRepo + Send + Sync>,
    response_repo: Arc<dyn ResponseRepo + Send + Sync>
) {
    // init instances
    let public_listener = TcpListener::bind(public_svc_address).await.unwrap();
    let client_listener = TcpListener::bind(client_svc_address).await.unwrap();
    let client_service = ClientService::new(client_repo);
    let public_service = PublicService::new(request_repo, response_repo);

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
