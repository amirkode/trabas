#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use common::config::{get_config_path, get_configs_from_dot_env, get_configs_from_proc_env, init_env_from_config, set_configs};


    fn get_test_env_path() -> String {
        let config_path = get_config_path();
        format!("{}/{}", config_path, ".env")
    }

    // create a mock .env file
    fn setup() {
        let config_path_str = get_config_path();
        let config_path = Path::new(&config_path_str);
        if !config_path.exists() {
            fs::create_dir_all(config_path).expect("Failed to create test config directory");
        }

        let env_path_str = get_test_env_path();
        let mut file = File::create(env_path_str).expect("Failed to create mock .env file");
        writeln!(file, "TEST_KEY_1=test_value_1").expect("Failed to write to mock .env file");
        writeln!(file, "TEST_KEY_2=test_value_2").expect("Failed to write to mock .env file");
    }

    // remove the mock .env file
    fn teardown() {
        let env_path = get_test_env_path();
        let env_path_exists = Path::new(&env_path).exists();
        if env_path_exists {
            fs::remove_file(env_path).expect("Failed to remove mock .env file");
        }
        let config_path = get_config_path();
        print!("dir: {}", config_path);
        if !env_path_exists && Path::new(&config_path).exists() {
           fs::remove_dir(config_path).expect("Failed to remove test config directory");
        }
    }

    #[test]
    fn test_get_configs_from_dot_env() {
        setup();
        let configs = get_configs_from_dot_env();
        let mut expected_configs = BTreeMap::new();
        expected_configs.insert("TEST_KEY_1".to_string(), "test_value_1".to_string());
        expected_configs.insert("TEST_KEY_2".to_string(), "test_value_2".to_string());
        assert_eq!(configs, expected_configs);
        teardown();
    }
    #[test]
    fn test_set_configs() {
        setup();
        let mut new_configs = HashMap::new();
        new_configs.insert("TEST_KEY_2".to_string(), "new_value_2".to_string());
        new_configs.insert("TEST_KEY_3".to_string(), "test_value_3".to_string());
        set_configs(new_configs);

        let configs_from_dot_env = get_configs_from_dot_env();
        let configs_from_proc_env = get_configs_from_proc_env();
        let mut expected_configs = BTreeMap::new();
        expected_configs.insert("TEST_KEY_1".to_string(), "test_value_1".to_string());
        expected_configs.insert("TEST_KEY_2".to_string(), "new_value_2".to_string());
        expected_configs.insert("TEST_KEY_3".to_string(), "test_value_3".to_string());

        // all keys and values in expected_confings should exist in configs_from_dot_nev and configs_from_proc_env
        for (key, value) in &expected_configs {
            assert_eq!(configs_from_dot_env.get(key), Some(value), "Key '{}' is missing or has a different value", key);
            assert_eq!(configs_from_proc_env.get(key), Some(value), "Key '{}' is missing or has a different value", key);
        }

        teardown();
    }

     #[test]
    fn test_init_env_from_config() {
        setup();
        init_env_from_config();

        assert_eq!(env::var("TEST_KEY_1").unwrap(), "test_value_1");
        assert_eq!(env::var("TEST_KEY_2").unwrap(), "test_value_2");
        teardown();
    }
}
