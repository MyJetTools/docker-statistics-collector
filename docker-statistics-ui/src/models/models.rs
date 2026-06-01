use std::collections::BTreeMap;

use serde::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PortHttpModel {
    pub ip: Option<String>,
    #[serde(rename = "privatePort")]
    pub private_port: u16,
    #[serde(rename = "publicPort")]
    pub public_port: Option<u16>,
    #[serde(rename = "portType")]
    pub port_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ContainerModel {
    pub id: String,
    pub image: String,
    pub names: Vec<String>,
    pub labels: Option<BTreeMap<String, String>>,
    pub enabled: bool,
    pub created: Option<i64>,
    /// Unix epoch seconds of the last container start. `None` when never
    /// started or the collector couldn't inspect it.
    #[serde(default)]
    pub started_at: Option<i64>,
    pub status: Option<String>,
    pub state: Option<String>,
    #[serde(default)]
    pub instance: String,
    pub cpu: CpuUsageJsonMode,
    pub mem: MemUsageJsonMode,
    #[serde(default)]
    pub files: FilesUsageJsonMode,
    #[serde(default)]
    pub net: NetUsageJsonMode,
    #[serde(default)]
    pub disk: DiskUsageJsonMode,
    pub cpu_usage_history: Option<Vec<f64>>,
    pub mem_usage_history: Option<Vec<i64>>,
    pub open_files_history: Option<Vec<i64>>,
    #[serde(default)]
    pub net_in_history: Option<Vec<f64>>,
    #[serde(default)]
    pub net_out_history: Option<Vec<f64>>,

    pub ports: Option<Vec<PortHttpModel>>,
    #[serde(default)]
    pub volumes: Option<Vec<VolumeHttpModel>>,
}

impl ContainerModel {
    pub fn filter_me(&self, value: &str) -> bool {
        if value == "" {
            return true;
        }

        if self.id.contains(value) {
            return true;
        }

        let value = value.to_lowercase();

        if self.image.to_lowercase().contains(&value) {
            return true;
        }

        for name in &self.names {
            if name.to_lowercase().contains(&value) {
                return true;
            }
        }

        if let Some(labels) = &self.labels {
            for (key, v) in labels {
                if key.to_lowercase().contains(&value) {
                    return true;
                }

                if v.to_lowercase().contains(&value) {
                    return true;
                }
            }
        }

        return false;
    }

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CpuUsageJsonMode {
    pub usage: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MemUsageJsonMode {
    pub usage: Option<i64>,
    pub available: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct FilesUsageJsonMode {
    /// File descriptors currently open by the container's main process.
    pub open: Option<i64>,
    /// `nofile` soft limit (`RLIMIT_NOFILE`) of the container's main process.
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct DiskUsageJsonMode {
    /// Writable-layer size in bytes (the container's own data on top of the
    /// image). `None` until the collector's first slow size pass.
    pub size_rw: Option<i64>,
    /// Total size in bytes including the image layers.
    pub size_root_fs: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct NetUsageJsonMode {
    /// Inbound throughput in MB/s. `None` until two collector samples exist.
    pub in_mbps: Option<f64>,
    /// Outbound throughput in MB/s.
    pub out_mbps: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MetricsByVm {
    pub vm: Option<String>,
    pub url: String,
    pub container: ContainerModel,
    /// Host RAM total of the VM this container runs on (bytes). `None` when the
    /// collector couldn't read `/proc/meminfo`. UI uses this as the effective
    /// limit when `container.mem.limit` is `None` (unlimited container).
    #[serde(default)]
    pub host_mem_total: Option<i64>,
}
