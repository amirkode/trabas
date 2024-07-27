use std::sync::Arc;

use config::validate_configs;
use data::repository::underlying_repo::UnderlyingRepoImpl;
use handler::main_handler::register_handler;
use log::info;
use service::underlying_service::UnderlyingService;

pub mod config;
pub mod data;
pub mod handler; 
pub mod service;

pub async fn serve(host: Option<String>, port: u16, use_tls: bool) {
    validate_configs();
    
    let underlying_host = match host {
        Some(h) => format!("{}:{}", h, port),
        None => format!("0.0.0.0:{}", port)
    };    
    
    // init instances]
    let underlying_repo = UnderlyingRepoImpl::new();
    let underlying_service = UnderlyingService::new(Arc::new(underlying_repo));

    // register handler
    register_handler(underlying_host, underlying_service, use_tls).await;

    info!("Client Service Stopped");
}
