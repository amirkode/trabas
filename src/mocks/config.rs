use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex;

use common::config::ConfigHandler;

pub struct MockConfigHandlerImpl {
    mock_configs: Arc<Mutex<BTreeMap<String, String>>>,
}

impl MockConfigHandlerImpl {
    pub fn new() -> Self {
        Self {
            mock_configs: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

#[async_trait]
impl ConfigHandler for MockConfigHandlerImpl {
    async fn get_configs(&self) -> BTreeMap<String, String> {
        let configs = self.mock_configs.lock().await;
        configs.clone()
    }

    async fn set_configs(&self, values: HashMap<String, String>) {
        let mut configs = self.mock_configs.lock().await;
        for (key, value) in values {
            configs.insert(key, value);
        }
    }
}
