use std::collections::BTreeMap;

use docker_sdk::list_of_containers::ContainerJsonModel;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct ServiceInfo {
    pub id: String,
    pub image: String,
    pub running: bool,
    pub mem_available: Option<i64>,
    pub mem_limit: Option<i64>,
    pub mem_usage: Option<i64>,
    pub cpu_usage: Option<f64>,
}

impl ServiceInfo {
    pub fn update(&mut self, info: &ContainerJsonModel) {
        if self.image != info.image {
            self.image = info.image.to_string();
        }
    }
}

pub struct ServicesCache {
    pub data: RwLock<BTreeMap<String, ServiceInfo>>,
}

impl ServicesCache {
    pub fn new() -> Self {
        ServicesCache {
            data: RwLock::new(BTreeMap::new()),
        }
    }

    pub async fn update_services(&self, infos: &[ContainerJsonModel]) {
        let mut write_access = self.data.write().await;

        for info in infos {
            if !write_access.contains_key(&info.id) {
                write_access.insert(
                    info.id.clone(),
                    ServiceInfo {
                        id: info.id.to_string(),
                        image: info.image.to_string(),
                        running: info.is_running(),
                        mem_available: None,
                        mem_usage: None,
                        cpu_usage: None,
                        mem_limit: None,
                    },
                );
            } else {
                let service_info = write_access.get_mut(&info.id).unwrap();
                service_info.update(info);
            }
        }
    }

    pub async fn update_usage(
        &self,
        id: &str,
        mem_usage: i64,
        mem_available: i64,
        mem_limit: i64,
        cpu_usage: f64,
    ) {
        let mut write_access = self.data.write().await;

        if let Some(container) = write_access.get_mut(id) {
            container.cpu_usage = Some(cpu_usage);
            container.mem_usage = Some(mem_usage);
            container.mem_limit = Some(mem_limit);
            container.mem_available = Some(mem_available);
        }
    }

    pub async fn reset_usage(&self, id: &str) {
        let mut write_access = self.data.write().await;
        if let Some(container) = write_access.get_mut(id) {
            container.mem_usage = None;
            container.mem_available = None;
            container.mem_limit = None;

            container.cpu_usage = None;
        }
    }

    pub async fn get_snapshot(&self) -> Vec<ServiceInfo> {
        let read_access = self.data.read().await;

        let mut result = Vec::new();

        for service_info in read_access.values() {
            result.push(service_info.clone());
        }

        result
    }
}
