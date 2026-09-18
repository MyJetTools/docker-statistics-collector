use std::collections::HashMap;

use docker_sdk::list_of_containers::ContainerJsonModel;

/// One container as the collector saw it during a single live Docker scan.
///
/// This is a pure snapshot DTO — the collector keeps no state between requests,
/// so everything here is what one pass over the Docker API produced. Values
/// that need two samples to be meaningful (network throughput) are shipped as
/// raw cumulative counters plus the instant they were read; deriving rates is
/// the API service's job, since it's the side that remembers previous polls.
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

    /// Cumulative received bytes across all interfaces, straight from Docker.
    /// `None` when the container isn't running or stats couldn't be read.
    pub net_rx_bytes: Option<i64>,
    /// Cumulative transmitted bytes across all interfaces.
    pub net_tx_bytes: Option<i64>,
    /// Unix milliseconds at which the counters above were sampled. The API
    /// needs it to turn two consecutive counter readings into MB/s.
    pub net_sampled_at_unix_ms: i64,

    /// Unix epoch seconds of the last container start (from `State.StartedAt`).
    /// `None` when never started or the inspect call failed.
    pub started_at: Option<i64>,

    /// File descriptors currently open by the container's main process.
    /// `None` when the host `/proc` is not reachable.
    pub open_files: Option<i64>,
    /// `nofile` soft limit (`RLIMIT_NOFILE`) of the container's main process.
    /// `None` when the host `/proc` is not reachable.
    pub fd_limit: Option<i64>,

    /// Writable-layer size in bytes. Filled from the background disk-size timer's
    /// cache, not measured during the scan — `None` until that timer has reached
    /// this container.
    pub size_rw: Option<i64>,
    /// Total size in bytes including the image layers. Same source.
    pub size_root_fs: Option<i64>,

    pub ports: Vec<ServiceInfoPortModel>,
    pub volumes: Vec<ServiceInfoVolumeModel>,
}

impl ServiceInfo {
    /// Everything a plain `GET /containers/json` entry carries — no stats yet.
    pub fn from_container_json(info: &ContainerJsonModel, sampled_at_unix_ms: i64) -> Self {
        Self {
            id: info.id.to_string(),
            image: info.image.to_string(),
            names: info.names.clone(),
            labels: info.labels.clone(),
            running: info.is_running(),
            created: info.created,
            state: info.state.clone(),
            status: info.status.clone(),
            mem_available: None,
            mem_limit: None,
            mem_usage: None,
            cpu_usage: None,
            net_rx_bytes: None,
            net_tx_bytes: None,
            net_sampled_at_unix_ms: sampled_at_unix_ms,
            started_at: None,
            open_files: None,
            fd_limit: None,
            size_rw: None,
            size_root_fs: None,
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
        }
    }

    /// `com.docker.compose.service` label, when the container carries one.
    pub fn compose_service(&self) -> Option<&str> {
        self.labels.as_ref()?.get("com.docker.compose.service").map(|v| v.as_str())
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
