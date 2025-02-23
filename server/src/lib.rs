pub mod data;
pub mod handler;
pub mod service;
pub mod types;
pub mod config;

use std::sync::Arc;
use common::_info;

use common::config::{ConfigHandler, ConfigHandlerImpl, keys::CONFIG_KEY_SERVER_REDIS_ENABLE};
use config::{validate_configs, get_cache_service};
use data::repository::cache_repo::{CacheRepo, CacheRepoRedisImpl, CacheRepoProcMemImpl};
use data::repository::client_repo::{ClientRepo, ClientRepoRedisImpl, ClientRepoProcMemImpl};
use data::repository::request_repo::{RequestRepo, RequestRepoRedisImpl, RequestRepoProcMemImpl};
use data::repository::response_repo::{ResponseRepo, ResponsRepoRedisImpl, ResponsRepoProcMemImpl};
use data::store::redis::RedisDataStore;
use handler::public_handler::register_public_handler;
use handler::tunnel_handler::register_tunnel_handler;
use service::client_service::ClientService;
use service::public_service::PublicService;

use tokio::net::TcpListener;

// TODO: too many parameters, bind it in a data struct
// entry point of the server service
pub async fn entry_point(
    root_host: String, 
    public_port: u16, 
    client_port: u16,
    client_request_limit: u16,
    cache_client_id: bool
) {
    // validate required configs
    let configs = validate_configs();
    let use_redis_default = "false".to_string();
    let use_redis = *configs.get(CONFIG_KEY_SERVER_REDIS_ENABLE).unwrap_or(&use_redis_default) == "true".to_string();
    let public_svc_address = format!("{}:{}", root_host, public_port);
    let client_svc_address = format!("{}:{}", root_host, client_port);

    // config handler
    let config_handler = Arc::new(ConfigHandlerImpl {});

    if use_redis {
        // store data in redis
        // TODO: add retrier if connection attempt is failed
        _info!("Redis: connecting...");
        let redis_store = RedisDataStore::new().unwrap();
        let redis_connection = redis_store.client.get_multiplexed_async_connection().await.unwrap();
        _info!("Redis: connected");

        // init repo to be injected
        let cache_repo = Arc::new(CacheRepoRedisImpl::new(redis_connection.clone()));
        let client_repo = Arc::new(ClientRepoRedisImpl::new(redis_connection.clone()));
        let request_repo = Arc::new(RequestRepoRedisImpl::new(redis_connection.clone()));
        let response_repo = Arc::new(ResponsRepoRedisImpl::new(redis_connection.clone()));
        // run the services
        run(
            public_svc_address,
            client_svc_address,
            client_request_limit,
            cache_client_id,
            cache_repo,
            client_repo,
            request_repo,
            response_repo,
            config_handler
        ).await;
    } else {
        // store data in trabas process
        // init repo to be injected
        let cache_repo = Arc::new(CacheRepoProcMemImpl::new());
        let client_repo = Arc::new(ClientRepoProcMemImpl::new());
        let request_repo = Arc::new(RequestRepoProcMemImpl::new());
        let response_repo = Arc::new(ResponsRepoProcMemImpl::new());
        // run the services
        run(
            public_svc_address,
            client_svc_address,
            client_request_limit,
            cache_client_id,
            cache_repo,
            client_repo,
            request_repo,
            response_repo,
            config_handler
        ).await;
    }
}

// TODO: implement app level TCP Listener with TLS for Client Connection
pub async fn run(
    public_svc_address: String,
    client_svc_address: String,
    client_request_limit: u16,
    cache_client_id: bool,
    cache_repo: Arc<dyn CacheRepo + Send + Sync>,
    client_repo: Arc<dyn ClientRepo + Send + Sync>,
    request_repo: Arc<dyn RequestRepo + Send + Sync>,
    response_repo: Arc<dyn ResponseRepo + Send + Sync>,
    config_handler: Arc<dyn ConfigHandler + Send + Sync>,
) {
    // init instances
    let public_listener = TcpListener::bind(public_svc_address).await.unwrap();
    let client_listener = TcpListener::bind(client_svc_address).await.unwrap();
    let cache_service = get_cache_service(cache_repo, config_handler);
    let client_service = ClientService::new(client_repo);
    let public_service = PublicService::new(request_repo, response_repo, client_request_limit);

    _info!("[Public Listener] Listening on: `{}`", public_listener.local_addr().unwrap());
    _info!("[Client Listener] Listening on: `{}`", client_listener.local_addr().unwrap());

    loop {
        tokio::select! {
            Ok((socket, _)) = public_listener.accept() => {
                register_public_handler(socket, client_service.clone(), public_service.clone(), cache_service.clone(), cache_client_id).await;
            }
            Ok((socket, _)) = client_listener.accept() => {
                register_tunnel_handler(socket, client_service.clone(), public_service.clone()).await;
            }
        }
    }
}
