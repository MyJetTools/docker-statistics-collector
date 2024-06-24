use std::collections::BTreeMap;

use tokio::sync::Mutex;

pub struct MetricsCache {
    cache: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl MetricsCache {
    pub fn new() -> Self {
        MetricsCache {
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    pub async fn update(&self, service_name: String, content: Vec<u8>) {
        let mut write_access = self.cache.lock().await;
        write_access.insert(service_name, content);
    }

    pub async fn get_list_of_services(&self) -> Vec<String> {
        let read_access = self.cache.lock().await;
        read_access.keys().cloned().collect()
    }
}
