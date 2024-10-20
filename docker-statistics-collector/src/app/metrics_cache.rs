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

    pub async fn get_content(&self, service_id: &str) -> Option<Vec<u8>> {
        let read_access: tokio::sync::MutexGuard<BTreeMap<String, Vec<u8>>> =
            self.cache.lock().await;
        read_access.get(service_id).cloned()
    }

    pub async fn get_aggregated_metrics(&self) -> Vec<u8> {
        let read_access = self.cache.lock().await;
        let mut result = Vec::new();
        for (_, value) in read_access.iter() {
            result.extend(value);
        }
        result
    }

    pub async fn get_sizes(&self) -> BTreeMap<String, usize> {
        let read_access = self.cache.lock().await;
        let mut result = BTreeMap::new();
        for (key, value) in read_access.iter() {
            result.insert(key.clone(), value.len());
        }
        result
    }
}
