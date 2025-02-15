use std::collections::HashMap;
use std::fs;

use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use mac_address::get_mac_address; 
use tokio_native_tls::native_tls::Certificate;

use common::config::*;

pub const CONFIG_CA_FILE_NAME: &str = "ca.crt";

// simple validation for config keys
pub fn validate_configs() {
    let config = get_configs_from_proc_env();
    let required_keys = [
        keys::CONFIG_KEY_CLIENT_ID,
        keys::CONFIG_KEY_CLIENT_SERVER_HOST,
        keys::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY
    ];
    for key in required_keys {
        if !config.contains_key(key) {
            panic!("{} config has not been set.", key)
        }
    }
}

pub fn generate_client_id(custom_id: Option<String>, force: bool) -> () {
    // check whether the client is already set
    let config = get_configs_from_proc_env();
    if config.contains_key(keys::CONFIG_KEY_CLIENT_ID) && !force {
        println!("Client ID is already generated, please check it in the config file. Consider using --force option to force regenerating");
        return;
    }

    let id = if custom_id.is_none() { 
        generate_client_id_from_mac_address(32)
    } else {
        custom_id.unwrap()
    };
    set_configs(HashMap::from([
        (String::from(keys::CONFIG_KEY_CLIENT_ID), id.clone())
    ]));

    println!("Client ID generated!");
    println!("Value: {}", id);
    println!("You may find the value later again in the config file")   
}

fn generate_client_id_from_mac_address(length: usize) -> String {
    // get device mac address
    let device_info = get_mac_address()
    .unwrap_or_else(|_| None)
    .map(|mac| mac.to_string())
    .unwrap_or_else(|| String::from("UNKNOWN"));

    // clean: remove special characters and convert to uppercase
    let clean_device_info: String = device_info
    .chars()
    .filter(|c| c.is_alphanumeric())
    .collect::<String>()
    .to_uppercase();

    // If clean_device_info is longer than the desired length, truncate it
    let base = if clean_device_info.len() > length {
        clean_device_info[..length].to_string()
    } else {
        clean_device_info
    };

    // length of random characters to add
    let random_length = length.saturating_sub(base.len());
    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(random_length)
        .map(char::from)
        .collect::<String>()
        .to_uppercase();

    format!("{}{}", base, random_string)
}

pub fn set_server_signing_key(value: String, force: bool) -> () {
    // check whether the signing key is already set
    let config = get_configs_from_proc_env();
    if config.contains_key(keys::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY) && !force {
        println!("Server Signing Key is already set, please check it in the config file. Consider using --force option to force resetting");
        return;
    }

    set_configs(HashMap::from([
        (String::from(keys::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY), value.clone())
    ]));

    println!("Server Signing Key has been set!");
    println!("Value: {}", value);
    println!("You may find the value later again in the config file")   
}

pub fn set_server_host(value: String, force: bool) -> () {
    // check whether the server host is already set
    let config = get_configs_from_proc_env();
    if config.contains_key(keys::CONFIG_KEY_CLIENT_SERVER_HOST) && !force {
        println!("Server Host is already set, please check it in the config file. Consider using --force option to force resetting");
        return;
    }

    set_configs(HashMap::from([
        (String::from(keys::CONFIG_KEY_CLIENT_SERVER_HOST), value.clone())
    ]));

    println!("Server Host has been set!");
    println!("Value: {}", value);
    println!("You may find the value later again in the config file")   
}

pub fn set_server_port(value: u16, force: bool) -> () {
    // check whether the server host is already set
    let config = get_configs_from_proc_env();
    if config.contains_key(keys::CONFIG_KEY_CLIENT_SERVER_PORT) && !force {
        println!("Server Port is already set, please check it in the config file. Consider using --force option to force resetting");
        return;
    }

    set_configs(HashMap::from([
        (String::from(keys::CONFIG_KEY_CLIENT_SERVER_PORT), format!("{}", value))
    ]));

    println!("Server Port has been set!");
    println!("Value: {}", value);
    println!("You may find the value later again in the config file")   
}

// get CA certificate for TLS connection
pub fn get_ca_certificate() -> Result<Certificate, String> {
    let config_path = get_config_path();
    let ca_path = format!("{}/{}", config_path, CONFIG_CA_FILE_NAME);
    let ca_data = fs::read(ca_path)
        .map_err(|e| format!("Error reading CA file: {}",  e))?;
    let ca = Certificate::from_pem(&ca_data)
        .map_err(|e| format!("Error loading Certificate: {}",  e))?;
    Ok(ca)
}
