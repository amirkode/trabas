use std::sync::Arc;

use common::_info;
use config::validate_configs;
use data::repository::underlying_repo::{UnderlyingRepo, UnderlyingRepoImpl};
use handler::main_handler::register_handler;
use service::underlying_service::UnderlyingService;

pub mod config;
pub mod data;
pub mod handler; 
pub mod service;

// TODO: too many parameters, bind it in a data struct
pub async fn entry_point(host: Option<String>, port: u16, use_tls: bool) {
    validate_configs();
    
    let underlying_svc_address = match host {
        Some(h) => format!("{}:{}", h, port),
        None => format!("127.0.0.1:{}", port)
    };    
    
    // init repo to be injected
    let underlying_repo = Arc::new(UnderlyingRepoImpl::new());
    
    // run the service
    serve(underlying_svc_address, underlying_repo, use_tls).await;
}

pub async fn serve(
    underlying_svc_address: String,
    underlying_repo: Arc<dyn UnderlyingRepo + Send + Sync>,
    use_tls: bool
) {
    let underlying_service = UnderlyingService::new(underlying_repo);

    // register handler
    register_handler(underlying_svc_address, underlying_service, use_tls).await;

    _info!("Client Service Stopped.");
}
