use std::{collections::HashMap, sync::Arc};

use common::{
    config::{get_config, set_configs}, data::dto::cache_config::CacheConfig, security::generate_hmac_key
};

use crate::{data::{repository::cache_repo::CacheRepoImpl, store::redis::RedisDataStore}, service::cache_service::{self, CacheService}};

pub const CONFIG_KEY_SERVER_SECRET: &str = "SV_SECRET";
pub const CONFIG_KEY_SERVER_REDIS_HOST: &str = "SV_REDIS_HOST";
pub const CONFIG_KEY_SERVER_REDIS_PORT: &str = "SV_REDIS_PORT";
pub const CONFIG_KEY_SERVER_REDIS_PASS: &str = "SV_REDIS_PASS";

// simple validation for config keys
pub fn validate_configs() {
    let config = get_config();
    let required_keys = [
        CONFIG_KEY_SERVER_SECRET,
        CONFIG_KEY_SERVER_REDIS_HOST,
        CONFIG_KEY_SERVER_REDIS_PORT,
        CONFIG_KEY_SERVER_REDIS_PASS
    ];
    for key in required_keys {
        if !config.contains_key(key) {
            panic!("{} config has not been set.", key)
        }
    }
}

pub fn generate_server_secret(force: bool) -> () {
    // check whether the secret is already set
    let config = get_config();
    if config.contains_key(CONFIG_KEY_SERVER_SECRET) && !force {
        println!("Server Secret is already generated, please check it in the config file. Consider using --force option to force regenerating");
        return;
    }

    let key = generate_hmac_key(32);
    set_configs(HashMap::from([
        (String::from(CONFIG_KEY_SERVER_SECRET), key.clone())
    ]));

    println!("Server Secret generated!");
    println!("Value: {}", key);
    println!("You may find the value later again in the config file")
}

pub fn set_redis_configs(key: Option<String>, host: Option<String>, port: Option<String>, pass: Option<String>, force: bool) -> () {
    let config = get_config();
    let mut config_to_set = HashMap::new();
    
    if let Some(k) = key {
        if config.contains_key(CONFIG_KEY_SERVER_SECRET) && !force {
            println!("Server secret is already set, please check it in the config file. Consider using --force option to force resetting");
            return;
        }

        config_to_set.insert(String::from(CONFIG_KEY_SERVER_SECRET), k);
    }

    if let Some(h) = host {
        if config.contains_key(CONFIG_KEY_SERVER_REDIS_HOST) && !force {
            println!("Redis Host is already set, please check it in the config file. Consider using --force option to force resetting");
            return;
        }

        config_to_set.insert(String::from(CONFIG_KEY_SERVER_REDIS_HOST), h);
    }

    if let Some(p) = port {
        if config.contains_key(CONFIG_KEY_SERVER_REDIS_PORT) && !force {
            println!("Redis Port is already set, please check it in the config file. Consider using --force option to force resetting");
            return;
        }

        config_to_set.insert(String::from(CONFIG_KEY_SERVER_REDIS_PORT), p);
    }

    if let Some(ps) = pass {
        if config.contains_key(CONFIG_KEY_SERVER_REDIS_PASS) && !force {
            println!("Redis Pass is already set, please check it in the config file. Consider using --force option to force resetting");
            return;
        }

        config_to_set.insert(String::from(CONFIG_KEY_SERVER_REDIS_PASS), ps);
    }

    set_configs(config_to_set);

    println!("Redis Configurations has been set!");
    println!("You may find the value later again in the config file"); 
}

// Cache Configs
async fn get_cache_service() -> CacheService {
    validate_configs();

    let redis_store = RedisDataStore::new().unwrap();
    let redis_connection = redis_store.client.get_multiplexed_async_connection().await.unwrap();
    let cache_repo = Arc::new(CacheRepoImpl::new(redis_connection.clone()));
    let cache_service = CacheService::new(cache_repo);

    cache_service
}

pub async fn set_cache_config(client_id: String, method: String, path: String, exp_duration: u32) {
    let cache_service = get_cache_service().await;

    cache_service.set_cache_config(CacheConfig::new(client_id.clone(), method.clone(), path.clone(), exp_duration)).await.unwrap();

    println!("Cache config has been set (Client ID: {}, Method: {}, Path: {}, Duration: {} seconds)", client_id, method, path, exp_duration);
}

pub async fn remove_cache_config(client_id: String, method: String, path: String) {
    let cache_service = get_cache_service().await;

    cache_service.remove_cache_config(client_id.clone(), method.clone(), path.clone()).await.unwrap();

    println!("Cache config has been unset (Client ID: {}, Method: {}, Path: {})", client_id, method, path);
}

pub async fn show_cache_config() {
    let cache_service = get_cache_service().await;

    cache_service.show_cache_config().await.unwrap();
}
