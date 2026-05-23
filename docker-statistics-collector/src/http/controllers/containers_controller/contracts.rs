use std::collections::HashMap;

use my_http_server::macros::MyHttpObjectStructure;
use serde::{Deserialize, Serialize};

use crate::app::ServiceInfo;
use crate::host_mem::HostMemSnapshot;

#[derive(MyHttpObjectStructure, Serialize, Deserialize)]
pub struct ContainersHttpResponse {
    pub vm: String,
    pub containers: Vec<ContainerJsonModel>,
    pub hosts: Vec<HostMemEntryHttpModel>,
}

// HostMemEntryHttpModel — per-instance host snapshot.
//   total/available/used: bytes from /proc/meminfo
//   cpu_count: logical processors from /proc/cpuinfo (0 = unknown)
// Field-level doc comments are intentionally omitted — MyHttpObjectStructure
// can't parse `#[doc="..."]` attributes (panics on the `=` punct).
#[derive(MyHttpObjectStructure, Serialize, Deserialize, Clone, Debug)]
pub struct HostMemEntryHttpModel {
    pub instance: String,
    pub total: i64,
    pub available: i64,
    pub used: i64,
    pub cpu_count: i32,
}

impl HostMemEntryHttpModel {
    pub fn from_snapshot(instance: String, s: HostMemSnapshot) -> Self {
        Self {
            instance,
            total: s.total,
            available: s.available,
            used: s.used,
            cpu_count: s.cpu_count.map(|v| v as i32).unwrap_or(0),
        }
    }
}

#[derive(Serialize, Deserialize, MyHttpObjectStructure)]
pub struct ContainerJsonModel {
    pub id: String,
    pub image: String,
    pub names: Vec<String>,
    pub labels: Option<HashMap<String, String>>,
    pub enabled: bool,
    pub created: i64,
    pub state: String,
    pub status: String,
    pub instance: String,
    pub cpu: CpuUsageJsonMode,
    pub mem: MemUsageJsonMode,
    #[serde(default)]
    pub files: FilesUsageJsonMode,
    pub ports: Vec<PortHttpModel>,
    pub volumes: Vec<VolumeHttpModel>,
}

impl ContainerJsonModel {
    pub fn new(itm: ServiceInfo, instance: String) -> Self {
        Self {
            id: itm.id,
            image: itm.image,
            enabled: itm.running,
            instance,
            cpu: CpuUsageJsonMode {
                usage: itm.cpu_usage,
            },
            mem: MemUsageJsonMode {
                usage: itm.mem_usage,
                available: itm.mem_available,
                limit: itm.mem_limit,
            },
            files: FilesUsageJsonMode {
                open: itm.open_files,
                limit: itm.fd_limit,
            },
            names: itm.names,
            labels: itm.labels,
            created: itm.created,

            state: itm.state,
            status: itm.status,

            ports: itm
                .ports
                .into_iter()
                .map(|itm| PortHttpModel {
                    ip: itm.ip,
                    private_port: itm.private_port,
                    public_port: itm.public_port,
                    port_type: itm.port_type,
                })
                .collect(),

            volumes: itm
                .volumes
                .into_iter()
                .map(|itm| VolumeHttpModel {
                    mount_type: itm.mount_type,
                    name: itm.name,
                    source: itm.source,
                    destination: itm.destination,
                    driver: itm.driver,
                    mode: itm.mode,
                    rw: itm.rw,
                    propagation: itm.propagation,
                })
                .collect(),
        }
    }

    pub fn into_service_info(self) -> ServiceInfo {
        use crate::app::{ServiceInfoPortModel, ServiceInfoVolumeModel};
        ServiceInfo {
            id: self.id,
            image: self.image,
            names: self.names,
            labels: self.labels,
            running: self.enabled,
            created: self.created,
            state: self.state,
            status: self.status,
            mem_available: self.mem.available,
            mem_limit: self.mem.limit,
            mem_usage: self.mem.usage,
            cpu_usage: self.cpu.usage,
            open_files: self.files.open,
            fd_limit: self.files.limit,
            ports: self
                .ports
                .into_iter()
                .map(|p| ServiceInfoPortModel {
                    ip: p.ip,
                    private_port: p.private_port,
                    public_port: p.public_port,
                    port_type: p.port_type,
                })
                .collect(),
            volumes: self
                .volumes
                .into_iter()
                .map(|v| ServiceInfoVolumeModel {
                    mount_type: v.mount_type,
                    name: v.name,
                    source: v.source,
                    destination: v.destination,
                    driver: v.driver,
                    mode: v.mode,
                    rw: v.rw,
                    propagation: v.propagation,
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, MyHttpObjectStructure)]
pub struct CpuUsageJsonMode {
    pub usage: Option<f64>,
}

#[derive(Serialize, Deserialize, MyHttpObjectStructure)]
pub struct MemUsageJsonMode {
    pub usage: Option<i64>,
    pub available: Option<i64>,
    pub limit: Option<i64>,
}

// `open`  — file descriptors currently open by the container's main process.
// `limit` — `nofile` soft limit (`RLIMIT_NOFILE`) of the container's main process.
#[derive(Serialize, Deserialize, MyHttpObjectStructure, Default)]
pub struct FilesUsageJsonMode {
    pub open: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, MyHttpObjectStructure)]
pub struct PortHttpModel {
    pub ip: Option<String>,
    #[serde(rename = "privatePort")]
    pub private_port: u16,
    #[serde(rename = "publicPort")]
    pub public_port: Option<u16>,
    #[serde(rename = "portType")]
    pub port_type: String,
}

#[derive(Serialize, Deserialize, MyHttpObjectStructure)]
pub struct ContainerProcessesHttpResponse {
    pub container_id: String,
    pub processes: Vec<ProcessHttpModel>,
}

// One process inside a container.
// `open_files` / `fd_limit` are None when the host `/proc` is not reachable.
#[derive(Serialize, Deserialize, MyHttpObjectStructure, Clone)]
pub struct ProcessHttpModel {
    pub pid: u32,
    pub cmd: String,
    pub open_files: Option<i64>,
    pub fd_limit: Option<i64>,
    pub mem_rss: Option<i64>,
    pub mem_vsize: Option<i64>,
    pub threads: Option<i64>,
}

#[derive(Serialize, Deserialize, MyHttpObjectStructure)]
pub struct VolumeHttpModel {
    #[serde(rename = "mountType")]
    pub mount_type: Option<String>,
    pub name: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub driver: Option<String>,
    pub mode: Option<String>,
    pub rw: Option<bool>,
    pub propagation: Option<String>,
}
