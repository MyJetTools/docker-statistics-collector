use std::{collections::BTreeMap, sync::Arc};

use tokio::sync::RwLock;

pub struct MetricsCache {
    cache: RwLock<BTreeMap<String, Arc<Vec<u8>>>>,
}

impl MetricsCache {
    pub fn new() -> Self {
        MetricsCache {
            cache: RwLock::new(BTreeMap::new()),
        }
    }

    pub async fn update(&self, service_name: String, content: Vec<u8>) {
        let mut write_access = self.cache.write().await;
        write_access.insert(service_name, Arc::new(content));
    }

    pub async fn get_list_of_services(&self) -> Vec<String> {
        let read_access = self.cache.read().await;
        read_access.keys().cloned().collect()
    }

    pub async fn get_content(&self, service_id: &str) -> Option<Arc<Vec<u8>>> {
        let read_access = self.cache.read().await;
        read_access.get(service_id).cloned()
    }

    pub async fn get_aggregated_metrics(&self) -> Vec<u8> {
        let read_access = self.cache.read().await;
        let total: usize = read_access.values().map(|v| v.len()).sum();
        let mut result = Vec::with_capacity(total);
        for value in read_access.values() {
            result.extend_from_slice(value);
        }
        result
    }

    pub async fn get_sizes(&self) -> BTreeMap<String, usize> {
        let read_access = self.cache.read().await;
        let mut result = BTreeMap::new();
        for (key, value) in read_access.iter() {
            result.insert(key.clone(), value.len());
        }
        result
    }
}
