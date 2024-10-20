use std::collections::{BTreeMap, HashMap};

use docker_sdk::list_of_containers::ContainerJsonModel;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct ServiceInfo {
    pub id: String,
    pub image: String,
    pub names: Vec<String>,
    pub labels: Option<HashMap<String, String>>,
    pub running: bool,
    pub created: i64,
    pub state: String,
    pub status: String,
    pub mem_available: Option<i64>,
    pub mem_limit: Option<i64>,
    pub mem_usage: Option<i64>,
    pub cpu_usage: Option<f64>,

    pub ports: Vec<ServiceInfoPortModel>,
}

impl ServiceInfo {
    pub fn update(&mut self, info: &ContainerJsonModel) {
        if self.image != info.image {
            self.image = info.image.to_string();
        }

        self.running = info.is_running();

        self.labels = info.labels.clone();
        self.names = info.names.clone();
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

        let mut to_remove = Vec::new();

        for container in write_access.values() {
            if !infos.into_iter().any(|itm| itm.id == container.id) {
                to_remove.push(container.id.to_string());
            }
        }

        for id in to_remove {
            write_access.remove(&id);
        }

        for info in infos {
            if !write_access.contains_key(&info.id) {
                write_access.insert(
                    info.id.clone(),
                    ServiceInfo {
                        id: info.id.to_string(),
                        image: info.image.to_string(),
                        names: info.names.clone(),
                        running: info.is_running(),
                        labels: info.labels.clone(),
                        created: info.created,
                        mem_available: None,
                        mem_usage: None,
                        cpu_usage: None,
                        mem_limit: None,
                        state: info.state.clone(),
                        status: info.status.clone(),
                        ports: match info.ports.as_ref() {
                            None => Vec::new(),
                            Some(ports) => ports
                                .iter()
                                .map(|itm| ServiceInfoPortModel {
                                    ip: itm.ip.clone(),
                                    private_port: itm.private_port,
                                    public_port: itm.public_port,
                                    port_type: itm.r#type.clone(),
                                })
                                .collect(),
                        },
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

#[derive(Clone, Debug)]
pub struct ServiceInfoPortModel {
    pub ip: Option<String>,
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub port_type: String,
}
