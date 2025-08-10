use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

// TODO: find proper approach to test these

// all config values in this tools will be written locally in this file
const CONFIG_PATH: &str = "trabas_config";
const CONFIG_ENV: &str = ".env";

// ALL VALID CONFIG CONSTANTS
pub mod keys {
    pub const CONFIG_KEY_GLOBAL_DEBUG: &str = "GLOBAL_DEBUG";
    pub const CONFIG_KEY_GLOBAL_LOG_LIMIT: &str = "GLOBAL_LOG_LIMIT";
    // client
    pub const CONFIG_KEY_CLIENT_ID: &str = "CL_ID";
    pub const CONFIG_KEY_CLIENT_SERVER_HOST: &str = "CL_SERVER_HOST";
    pub const CONFIG_KEY_CLIENT_SERVER_PORT: &str = "CL_SERVER_PORT";
    pub const CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY: &str = "CL_SERVER_SIGNING_KEY";
    // server
    pub const CONFIG_KEY_SERVER_SECRET: &str = "SV_SECRET";
    pub const CONFIG_KEY_SERVER_ENFORCE_TLS: &str = "SV_ENFORCE_TLS";
    pub const CONFIG_KEY_SERVER_PUBLIC_ENDPOINT: &str = "SV_PUBLIC_ENDPOINT";
    pub const CONFIG_KEY_SERVER_PUBLIC_REQUEST_TIMEOUT: &str = "SV_PUBLIC_REQUEST_TIMEOUT";
    pub const CONFIG_KEY_SERVER_CACHE_CONFIGS: &str = "SV_CACHE_CONFIGS";
    pub const CONFIG_KEY_SERVER_REDIS_ENABLE: &str = "SV_REDIS_ENABLE";
    pub const CONFIG_KEY_SERVER_REDIS_HOST: &str = "SV_REDIS_HOST";
    pub const CONFIG_KEY_SERVER_REDIS_PORT: &str = "SV_REDIS_PORT";
    pub const CONFIG_KEY_SERVER_REDIS_PASS: &str = "SV_REDIS_PASS";
}

// TODO: should we make a config interface for both client and server (?)
// and to be injected across the usecases (?)
// for now, sharing config/env using std::env is a decent solution

pub fn get_config_path() -> String {
    let root_path = env::current_exe().unwrap().parent().unwrap().to_string_lossy().to_string();
    format!("{}/{}", root_path, CONFIG_PATH)
}

fn get_env_path() -> String {
    let config_path = get_config_path();
    format!("{}/{}", config_path, CONFIG_ENV)
}

// load all configs from .env file into map
// using BTreeMap to get ordered keys
pub fn get_configs_from_dot_env() -> BTreeMap<String, String> {
    let env_path = get_env_path();
    let file = File::open(env_path);
    let mut map = BTreeMap::new();
    if file.is_err() {
        // .env file could not be found 
        return map;
    }

    let reader = BufReader::new(file.unwrap());
    for line in reader.lines() {
        if line.is_err() {
            // skip cannot read line
            continue;
        }

        let line = line.unwrap();
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            map.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
        }
    }

    map
}

pub fn get_configs_from_proc_env() -> BTreeMap<String, String> {
    env::vars().collect()
}

pub fn set_configs(values: HashMap<String, String>) {
    // make sure the path is exists
    let config_path_str = get_config_path();
    let config_path = Path::new(config_path_str.as_str());
    if !config_path.exists() {
        create_dir_all(config_path).expect("Unable to initiate config directory.");
    }
    
    // fecth existing env vars
    let mut config = get_configs_from_dot_env();
    for (key, value) in values {
        config.insert(key, value);
    }

    let env_path = get_env_path();
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(env_path).expect(format!("Unable to open {} file for writing.", CONFIG_ENV).as_str());

    // update entire file
    for (key, value) in config {
        // update env
        env::set_var(key.clone(), value.clone());
        writeln!(file, "{}={}", key, value).expect(format!("Unable to write to {} file.", CONFIG_ENV).as_str())
    }
}

pub fn init_env_from_config() {
    let env_path = get_env_path();
    dotenv::from_filename(env_path).ok();
}

// this wraps base config functions
#[async_trait]
pub trait ConfigHandler {
    async fn get_configs(&self) -> BTreeMap<String, String>;
    async fn set_configs(&self, values: HashMap<String, String>);
}

pub struct ConfigHandlerImpl;

#[async_trait]
impl ConfigHandler for ConfigHandlerImpl {
    async fn get_configs(&self) -> BTreeMap<String, String> {
        get_configs_from_dot_env()
    }

    async fn set_configs(&self, values: HashMap<String, String>) {
        set_configs(values);
    }
}
