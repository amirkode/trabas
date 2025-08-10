pub mod data;
pub mod handler;
pub mod service;
pub mod types;
pub mod config;
pub mod version;

use common::_info;

use common::config::{ConfigHandler, ConfigHandlerImpl, keys::CONFIG_KEY_SERVER_REDIS_ENABLE};
use config::{ServerRequestConfig, get_server_identity_from_pem, validate_configs, get_cache_service};
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
use redis::aio::MultiplexedConnection;

// TLS
use tokio_native_tls::TlsAcceptor as TokioTlsAcceptor;
use native_tls::{Identity, TlsAcceptor};
use std::path::PathBuf;
use common::net::TcpStreamTLS;

// entry point of the server service
pub async fn entry_point(config: ServerRequestConfig) {
    // validate required configs
    let configs = validate_configs();
    let use_redis_default = "false".to_string();
    let use_redis = *configs.get(CONFIG_KEY_SERVER_REDIS_ENABLE).unwrap_or(&use_redis_default) == "true".to_string();

    // config handler
    let config_handler = std::sync::Arc::new(ConfigHandlerImpl {});

    if use_redis {
        // store data in redis
        let mut redis_connection: Option<MultiplexedConnection> = None;
        let tries = 5; // 5 attempts with 2 seconds delay each
        while tries > 0 {
            let redis_store = RedisDataStore::new();
            if redis_store.is_err() {
                _info!("Redis connection failed, retrying...");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }

            let connection = redis_store.unwrap().client.get_multiplexed_async_connection().await;
            if connection.is_err() {
                _info!("Redis connection failed, retrying...");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }

            redis_connection = Some(connection.unwrap());
        }

        if redis_connection.is_none() {
            panic!("Failed to connect to Redis after multiple attempts.");
        }

        let redis_connection = redis_connection.unwrap();

        // init repo to be injected
        let cache_repo = std::sync::Arc::new(CacheRepoRedisImpl::new(redis_connection.clone()));
        let client_repo = std::sync::Arc::new(ClientRepoRedisImpl::new(redis_connection.clone()));
        let request_repo = std::sync::Arc::new(RequestRepoRedisImpl::new(redis_connection.clone()));
        let response_repo = std::sync::Arc::new(ResponsRepoRedisImpl::new(redis_connection.clone()));
        // run the services
        run(
            config,
            cache_repo,
            client_repo,
            request_repo,
            response_repo,
            config_handler
        ).await;
    } else {
        // store data in trabas process
        // init repo to be injected
        let cache_repo = std::sync::Arc::new(CacheRepoProcMemImpl::new());
        let client_repo = std::sync::Arc::new(ClientRepoProcMemImpl::new());
        let request_repo = std::sync::Arc::new(RequestRepoProcMemImpl::new());
        let response_repo = std::sync::Arc::new(ResponsRepoProcMemImpl::new());
        // run the services
        run(
            config,
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
    config: ServerRequestConfig,
    cache_repo: std::sync::Arc<dyn CacheRepo + Send + Sync>,
    client_repo: std::sync::Arc<dyn ClientRepo + Send + Sync>,
    request_repo: std::sync::Arc<dyn RequestRepo + Send + Sync>,
    response_repo: std::sync::Arc<dyn ResponseRepo + Send + Sync>,
    config_handler: std::sync::Arc<dyn ConfigHandler + Send + Sync>,
) {
    // init instances
    let public_listener = TcpListener::bind(config.public_svc_address()).await.unwrap();
    let client_listener = TcpListener::bind(config.client_svc_address()).await.unwrap();

    _info!("[Public Listener] Listening on: `{}`", public_listener.local_addr().unwrap());
    _info!("[Client Listener] Listening on: `{}`", client_listener.local_addr().unwrap());

    let cache_service = get_cache_service(cache_repo, config_handler);
    let client_service = ClientService::new(client_repo);
    let public_service = PublicService::new(request_repo, response_repo, config.client_request_limit);
    let enforce_tls = std::env::var(common::config::keys::CONFIG_KEY_SERVER_ENFORCE_TLS)
        .unwrap_or_else(|_| "false".into())
        .to_lowercase() == "true";
    let tls_acceptor: Option<TokioTlsAcceptor> = if enforce_tls {
        match build_tls_acceptor() {
            Ok(a) => Some(a),
            Err(e) => {
                panic!("Failed to initialize TLS acceptor: {}", e);
            }
        }
    } else { None };

    loop {
        tokio::select! {
            Ok((socket, _)) = public_listener.accept() => {
                register_public_handler(
                    socket, 
                    client_service.clone(), 
                    public_service.clone(), 
                    cache_service.clone(), 
                    config.cache_client_id,
                    config.return_tunnel_id
                ).await;
            }
            Ok((socket, _)) = client_listener.accept() => {
                if let Some(ref acceptor) = tls_acceptor {
                    _info!("[Client Listener] Accepting connections with TLS...");

                    let s = socket;
                    let acceptor = acceptor.clone();
                    let cs = client_service.clone();
                    let ps = public_service.clone();
                    tokio::spawn(async move {
                        match acceptor.accept(s).await {
                            Ok(tls_stream) => {
                                let (r, w) = tokio::io::split(tls_stream);
                                let read = TcpStreamTLS::from_tcp_tls_read(r);
                                let write = TcpStreamTLS::from_tcp_tls_write(w);
                                crate::handler::tunnel_handler::register_tunnel_handler(read, write, cs, ps).await;
                            }
                            Err(e) => {
                                _info!("TLS handshake failed: {}", e);
                            }
                        }
                    });
                } else {
                    // without TLS
                    let (r, w) = tokio::io::split(socket);
                    let read = TcpStreamTLS::from_tcp_read(r);
                    let write = TcpStreamTLS::from_tcp_write(w);
                    register_tunnel_handler(read, write, client_service.clone(), public_service.clone()).await;
                }
            }
        }
    }
}

fn build_tls_acceptor() -> Result<TokioTlsAcceptor, String> {
    let identity = get_server_identity_from_pem()?;
    let acceptor = TlsAcceptor::builder(identity).build().map_err(|e| format!("build TlsAcceptor: {}", e))?;
    
    Ok(TokioTlsAcceptor::from(acceptor))
}
