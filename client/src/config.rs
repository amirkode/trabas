use std::collections::HashMap;

use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use mac_address::get_mac_address; 

use common::config::{get_config, set_configs};

const CONFIG_KEY_CLIENT_ID: &str = "CL_ID";
const CONFIG_KEY_CLIENT_SERVER_HOST: &str = "CL_SERVER_HOST";
const CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY: &str = "CL_SERVER_SIGNING_KEY";

pub fn generate_client_id(custom_id: Option<String>, force: bool) -> () {
    // check whether the client is already set
    let config = get_config();
    if config.contains_key(CONFIG_KEY_CLIENT_ID) && !force {
        println!("Client ID is already generated, please check it in the config file. Consider using --force option to force regenerating");
        return;
    }

    let id = if custom_id.is_none() { 
        generate_client_id_from_mac_address(32)
    } else {
        custom_id.unwrap()
    };
    set_configs(HashMap::from([
        (String::from(CONFIG_KEY_CLIENT_ID), id.clone())
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
    let config = get_config();
    if config.contains_key(CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY) && !force {
        println!("Server Signing Key is already set, please check it in the config file. Consider using --force option to force resetting");
        return;
    }

    set_configs(HashMap::from([
        (String::from(CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY), value.clone())
    ]));

    println!("Server Signing Key has been set!");
    println!("Value: {}", value);
    println!("You may find the value later again in the config file")   
}

pub fn set_server_host(value: String, force: bool) -> () {
    // check whether the server host is already set
    let config = get_config();
    if config.contains_key(CONFIG_KEY_CLIENT_SERVER_HOST) && !force {
        println!("Server Host is already set, please check it in the config file. Consider using --force option to force resetting");
        return;
    }

    set_configs(HashMap::from([
        (String::from(CONFIG_KEY_CLIENT_SERVER_HOST), value.clone())
    ]));

    println!("Server Host has been set!");
    println!("Value: {}", value);
    println!("You may find the value later again in the config file")   
}
