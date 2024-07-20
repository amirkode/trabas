use std::collections::HashMap;

use common::{
    config::{get_config, set_configs}, 
    security::generate_hmac_key,
};

const CONFIG_KEY_SERVER_SECRET: &str = "SV_SECRET";

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
