use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

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

    /// Network throughput in MB/s, derived from the delta of cumulative
    /// rx/tx byte counters between two stats polls. `None` until we have two
    /// samples.
    pub net_in_mbps: Option<f64>,
    pub net_out_mbps: Option<f64>,
    /// Previous cumulative counters + the instant they were taken, used to
    /// compute the rate on the next poll.
    pub(crate) prev_rx_bytes: Option<i64>,
    pub(crate) prev_tx_bytes: Option<i64>,
    pub(crate) prev_net_at: Option<Instant>,

    /// Unix epoch seconds of the last container start (from
    /// `State.StartedAt`). `None` when never started or not inspected yet.
    pub started_at: Option<i64>,

    /// File descriptors currently open by the container's main process.
    /// `None` when the host `/proc` is not reachable.
    pub open_files: Option<i64>,
    /// `nofile` soft limit (`RLIMIT_NOFILE`) of the container's main process.
    /// `None` when the host `/proc` is not reachable.
    pub fd_limit: Option<i64>,

    /// Writable-layer disk usage in bytes (the container's own data on top of
    /// the image). Refreshed on a slow cadence — see the size timer. `None`
    /// until the first size pass completes.
    pub size_rw: Option<i64>,
    /// Total disk size in bytes including image layers. Slow-cadence too.
    pub size_root_fs: Option<i64>,

    pub ports: Vec<ServiceInfoPortModel>,
    pub volumes: Vec<ServiceInfoVolumeModel>,
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
                        net_in_mbps: None,
                        net_out_mbps: None,
                        prev_rx_bytes: None,
                        prev_tx_bytes: None,
                        prev_net_at: None,
                        open_files: None,
                        fd_limit: None,
                        size_rw: None,
                        size_root_fs: None,
                        started_at: None,
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
                        volumes: match info.mounts.as_ref() {
                            None => Vec::new(),
                            Some(mounts) => mounts
                                .iter()
                                .map(|itm| ServiceInfoVolumeModel {
                                    mount_type: itm.mount_type.clone(),
                                    name: itm.name.clone(),
                                    source: itm.source.clone(),
                                    destination: itm.destination.clone(),
                                    driver: itm.driver.clone(),
                                    mode: itm.mode.clone(),
                                    rw: itm.rw,
                                    propagation: itm.propagation.clone(),
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
        rx_bytes: i64,
        tx_bytes: i64,
    ) {
        let mut write_access = self.data.write().await;

        if let Some(container) = write_access.get_mut(id) {
            container.cpu_usage = Some(cpu_usage);
            container.mem_usage = Some(mem_usage);
            container.mem_limit = Some(mem_limit);
            container.mem_available = Some(mem_available);

            // Derive MB/s from the delta since the previous sample.
            let now = Instant::now();
            if let (Some(prev_rx), Some(prev_tx), Some(prev_at)) = (
                container.prev_rx_bytes,
                container.prev_tx_bytes,
                container.prev_net_at,
            ) {
                let secs = now.duration_since(prev_at).as_secs_f64();
                if secs > 0.0 {
                    const MB: f64 = 1024.0 * 1024.0;
                    // Counters reset to a lower value on container restart — clamp
                    // negatives to 0 instead of reporting a huge spike.
                    let rx_rate = (rx_bytes - prev_rx).max(0) as f64 / secs / MB;
                    let tx_rate = (tx_bytes - prev_tx).max(0) as f64 / secs / MB;
                    container.net_in_mbps = Some(rx_rate);
                    container.net_out_mbps = Some(tx_rate);
                }
            }
            container.prev_rx_bytes = Some(rx_bytes);
            container.prev_tx_bytes = Some(tx_bytes);
            container.prev_net_at = Some(now);
        }
    }

    pub async fn update_fd_usage(&self, id: &str, open_files: Option<i64>, fd_limit: Option<i64>) {
        let mut write_access = self.data.write().await;

        if let Some(container) = write_access.get_mut(id) {
            container.open_files = open_files;
            container.fd_limit = fd_limit;
        }
    }

    /// Slow-cadence disk-usage update. Kept separate from `update_usage` /
    /// `reset_usage` so the cached sizes survive between the infrequent size
    /// passes and aren't wiped on every 5s stats tick.
    pub async fn update_disk_usage(
        &self,
        id: &str,
        size_rw: Option<i64>,
        size_root_fs: Option<i64>,
    ) {
        let mut write_access = self.data.write().await;
        if let Some(container) = write_access.get_mut(id) {
            container.size_rw = size_rw;
            container.size_root_fs = size_root_fs;
        }
    }

    pub async fn update_started_at(&self, id: &str, started_at: Option<i64>) {
        let mut write_access = self.data.write().await;
        if let Some(container) = write_access.get_mut(id) {
            container.started_at = started_at;
        }
    }

    pub async fn reset_usage(&self, id: &str) {
        let mut write_access = self.data.write().await;
        if let Some(container) = write_access.get_mut(id) {
            container.mem_usage = None;
            container.mem_available = None;
            container.mem_limit = None;

            container.cpu_usage = None;

            container.net_in_mbps = None;
            container.net_out_mbps = None;
            container.prev_rx_bytes = None;
            container.prev_tx_bytes = None;
            container.prev_net_at = None;

            container.open_files = None;
            container.fd_limit = None;

            // Keep started_at intact across reset — it's a property of the
            // last running session, not the live stats. Stops it from flicking
            // to None whenever a stats fetch glitches.
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

#[derive(Clone, Debug)]
pub struct ServiceInfoVolumeModel {
    pub mount_type: Option<String>,
    pub name: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub driver: Option<String>,
    pub mode: Option<String>,
    pub rw: Option<bool>,
    pub propagation: Option<String>,
}
