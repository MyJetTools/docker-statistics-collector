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
    pub net_in_history: Option<Vec<f64>>,
    pub net_out_history: Option<Vec<f64>>,

    pub ports: Option<Vec<PortHttpModel>>,
    #[serde(default)]
    pub volumes: Option<Vec<VolumeHttpModel>>,
}

impl ContainerModel {
    pub fn update(&mut self, src: ContainerJsonModel) {
        self.cpu = src.cpu;
        self.mem = src.mem;
        self.files = src.files;
        self.net = src.net;
        self.disk = src.disk;
        self.labels = src.labels;
        self.enabled = src.enabled;
        self.image = src.image;
        self.instance = src.instance;
        // Adopt new started_at only when it's known; keep the previous value
        // across transient inspect glitches (same pattern as collector cache).
        if src.started_at.is_some() {
            self.started_at = src.started_at;
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ContainerJsonModel {
    pub id: String,
    pub image: String,
    pub names: Vec<String>,
    pub labels: Option<BTreeMap<String, String>>,
    pub enabled: bool,
    pub created: Option<i64>,
    #[serde(default)]
    pub started_at: Option<i64>,
    pub state: Option<String>,
    pub status: Option<String>,
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
    pub ports: Option<Vec<PortHttpModel>>,
    #[serde(default)]
    pub volumes: Option<Vec<VolumeHttpModel>>,
}

#[derive(Serialize, Deserialize)]
pub struct StatisticsContract {
    pub vm: String,
    pub containers: Vec<ContainerJsonModel>,
    #[serde(default)]
    pub hosts: Vec<HostMemEntryModel>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HostMemEntryModel {
    pub instance: String,
    pub total: i64,
    pub available: i64,
    pub used: i64,
    /// Logical CPU count of the host. `0` means unknown.
    #[serde(default)]
    pub cpu_count: i32,
    /// Physical disks on the host. Empty when the host root filesystem is not
    /// bind-mounted into the collector container.
    #[serde(default)]
    pub disks: Vec<DiskModel>,
}

/// One physical filesystem on the host (mirrors the collector's
/// `HostDiskHttpModel`). Sizes are bytes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DiskModel {
    pub device: String,
    #[serde(rename = "mountPoint")]
    pub mount_point: String,
    #[serde(rename = "fsType")]
    pub fs_type: String,
    pub total: i64,
    pub used: i64,
    pub available: i64,
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
pub struct NetUsageJsonMode {
    /// Inbound throughput in MB/s. `None` until the collector has two samples.
    pub in_mbps: Option<f64>,
    /// Outbound throughput in MB/s.
    pub out_mbps: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct DiskUsageJsonMode {
    /// Writable-layer size in bytes (the container's own data on top of the
    /// image). `None` until the collector's first slow size pass.
    pub size_rw: Option<i64>,
    /// Total size in bytes including the image layers.
    pub size_root_fs: Option<i64>,
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
