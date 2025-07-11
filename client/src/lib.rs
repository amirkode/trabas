use std::sync::Arc;

use common::_info;
use config::{ClientRequestConfig, validate_configs};
use data::repository::underlying_repo::{UnderlyingRepo, UnderlyingRepoImpl};
use handler::main_handler::register_handler;
use service::underlying_service::UnderlyingService;

pub mod config;
pub mod data;
pub mod handler; 
pub mod service;
pub mod version;

pub async fn entry_point(config: ClientRequestConfig) {
    validate_configs();
    
    // init repo to be injected
    let underlying_repo = Arc::new(UnderlyingRepoImpl::new());
    
    // run the service
    serve(config.underlying_svc_address(), underlying_repo, config.use_tls).await;
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
