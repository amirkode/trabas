use std::collections::HashMap;

use common::{
    config::{get_config, set_configs}, 
    security::generate_hmac_key,
};

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

pub fn set_redis_configs(host: Option<String>, port: Option<String>, pass: Option<String>, force: bool) -> () {
    let config = get_config();
    let mut config_to_set = HashMap::new();
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
