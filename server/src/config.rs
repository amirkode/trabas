use std::{collections::HashMap, sync::Arc};

use common::{
    config::*, 
    data::dto::cache_config::CacheConfig, 
    security::generate_hmac_key
};

use crate::{
    data::repository::cache_repo::{CacheRepo, CacheRepoProcMemImpl}, 
    service::cache_service::CacheService
};

#[derive(Debug, Clone)]
pub struct ServerRequestConfig {
    pub host: String, 
    pub public_port: u16, 
    pub client_port: u16,
    pub client_request_limit: u16,
    pub cache_client_id: bool,
    pub return_tunnel_id: bool
}

impl ServerRequestConfig {
    pub fn new(
        host: String, 
        public_port: u16, 
        client_port: u16, 
        client_request_limit: u16, 
        cache_client_id: bool,
        return_tunnel_id: bool
    ) -> Self {
        ServerRequestConfig {
            host,
            public_port,
            client_port,
            client_request_limit,
            cache_client_id,
            return_tunnel_id
        }
    }

    pub fn public_svc_address(&self) -> String {
        format!("{}:{}", self.host, self.public_port)
    }

    pub fn client_svc_address(&self) -> String {
        format!("{}:{}", self.host, self.client_port)
    }
}


// simple validation for config keys
pub fn validate_configs() -> HashMap<String, String> {
    let config = get_configs_from_proc_env();
    let use_redis_default = "false".to_string();
    let use_redis = *config.get(keys::CONFIG_KEY_SERVER_REDIS_ENABLE).unwrap_or(&use_redis_default) == "true".to_string();
    let mut required_keys = [
        keys::CONFIG_KEY_SERVER_SECRET,
    ].to_vec();
    if use_redis {
        // here we must define define redis configuration
        required_keys.extend([
            keys::CONFIG_KEY_SERVER_REDIS_HOST,
            keys::CONFIG_KEY_SERVER_REDIS_PORT,
            keys::CONFIG_KEY_SERVER_REDIS_PASS
        ])
    }
    for key in required_keys {
        if !config.contains_key(key) {
            panic!("{} config has not been set.", key)
        }
    }

    // returning required config values to the caller
    let mut res = HashMap::new();
    res.insert(keys::CONFIG_KEY_SERVER_REDIS_ENABLE.to_string(), if use_redis { "true".to_string() } else { "false".to_string() });

    res
}

pub fn generate_server_secret(force: bool) -> () {
    // check whether the secret is already set
    let config = get_configs_from_proc_env();
    if config.contains_key(keys::CONFIG_KEY_SERVER_SECRET) && !force {
        println!("Server Secret is already generated, please check it in the config file. Consider using --force option to force regenerating");
        return;
    }

    let key = generate_hmac_key(32);
    set_configs(HashMap::from([
        (String::from(keys::CONFIG_KEY_SERVER_SECRET), key.clone())
    ]));

    println!("Server Secret generated!");
    println!("Value: {}", key);
    println!("You may find the value later again in the config file")
}

pub fn set_server_configs(
    key: Option<String>,
    redis_enable: Option<String>,
    redis_host: Option<String>,
    redis_port: Option<String>,
    redis_pass: Option<String>,
    public_endpoint: Option<String>,
    public_request_timeout: Option<String>,
    force: bool,
) -> () {
    let config = get_configs_from_proc_env();
    let mut config_to_set = HashMap::new();

    #[derive(PartialEq, Copy, Clone)]
    enum ValueType { Int }
    let key_types: HashMap<&str, ValueType> = [
        (keys::CONFIG_KEY_SERVER_REDIS_PORT, ValueType::Int),
        (keys::CONFIG_KEY_SERVER_PUBLIC_REQUEST_TIMEOUT, ValueType::Int),
        // TODO: add more types as needed
    ].iter().map(|(k, v)| (*k, *v)).collect();

    let config_options = [
        (redis_enable, keys::CONFIG_KEY_SERVER_REDIS_ENABLE, "Redis enable flag"),
        (key, keys::CONFIG_KEY_SERVER_SECRET, "Server secret"),
        (redis_host, keys::CONFIG_KEY_SERVER_REDIS_HOST, "Redis Host"),
        (redis_port, keys::CONFIG_KEY_SERVER_REDIS_PORT, "Redis Port"),
        (redis_pass, keys::CONFIG_KEY_SERVER_REDIS_PASS, "Redis Pass"),
        (public_endpoint, keys::CONFIG_KEY_SERVER_PUBLIC_ENDPOINT, "Public Endpoint"),
        (public_request_timeout, keys::CONFIG_KEY_SERVER_PUBLIC_REQUEST_TIMEOUT, "Public Request Timeout"),
    ];

    for (opt, key_str, msg) in config_options.iter() {
        if let Some(val) = opt {
            // validate the type
            match key_types.get(key_str) {
                Some(ValueType::Int) => {
                    if val.parse::<u32>().is_err() {
                        println!("{msg} must be an integer value.");
                        return;
                    }
                }
                // TODO: might add boolean
                _ => {}
            }
            if config.contains_key(*key_str) && !force {
                println!("{msg} is already set, please check it in the config file. Consider using --force option to force resetting");
                return;
            }
            config_to_set.insert(String::from(*key_str), val.clone());
        }
    }

    set_configs(config_to_set);

    println!("Server Configurations have been set!");
    println!("You may find the value later again in the config file");
}

// Cache Configs
pub fn get_cache_service(
        cache_repo: Arc<dyn CacheRepo + Send + Sync>, 
        config_handler: Arc<dyn ConfigHandler + Send + Sync>) -> CacheService {
    let cache_service = CacheService::new(
        cache_repo, 
        config_handler,
        String::from(keys::CONFIG_KEY_SERVER_CACHE_CONFIGS)
    );

    cache_service
}

fn get_cache_service_for_settings() -> CacheService {
    validate_configs();
    // use in process memo repo since we don't really need this
    // TODO: might consider separate cache & cache config in different services
    let cache_repo = Arc::new(CacheRepoProcMemImpl::new());
    let config_handler = Arc::new(ConfigHandlerImpl{});
    
    get_cache_service(cache_repo, config_handler)
}

pub async fn set_cache_config(client_id: String, method: String, path: String, exp_duration: u32) {
    let cache_service = get_cache_service_for_settings();
    cache_service.set_cache_config(CacheConfig::new(client_id.clone(), method.clone(), path.clone(), exp_duration)).await.unwrap();

    println!("Cache config has been set (Client ID: {}, Method: {}, Path: {}, Duration: {} seconds)", client_id, method, path, exp_duration);
}

pub async fn remove_cache_config(client_id: String, method: String, path: String) {
    let cache_service = get_cache_service_for_settings();

    cache_service.remove_cache_config(client_id.clone(), method.clone(), path.clone()).await.unwrap();

    println!("Cache config has been unset (Client ID: {}, Method: {}, Path: {})", client_id, method, path);
}

pub async fn show_cache_config() {
    let cache_service = get_cache_service_for_settings();

    cache_service.show_cache_config().await.unwrap();
}
