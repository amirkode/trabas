use std::sync::Arc;

use config::validate_configs;
use data::repository::underlying_repo::UnderlyingRepoImpl;
use handler::main_handler::register_handler;
use service::underlying_service::UnderlyingService;
use tokio::net::TcpStream;

pub mod config;
pub mod data;
pub mod handler; 
pub mod service;

pub async fn serve(host: Option<String>, port: u16) {
    validate_configs();
    
    let underlying_host = match host {
        Some(h) => format!("{}:{}", h, port),
        None => format!("0.0.0.0:{}", port)
    };
    // init instances
    let stream = TcpStream::connect(underlying_host.clone()).await.unwrap();
    let underlying_repo = UnderlyingRepoImpl::new();
    let underlying_service = UnderlyingService::new(Arc::new(underlying_repo));

    // register handler
    register_handler(stream, underlying_host, underlying_service).await;
}
