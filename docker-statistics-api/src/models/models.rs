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
    /// Previous raw network counters as reported by the collector. The
    /// collector is stateless, so this service is what turns two consecutive
    /// readings into the MB/s above. Never leaves this process.
    #[serde(skip)]
    pub net_prev: Option<NetSample>,
    /// Disk usage in bytes, as measured by the collector's background size timer and
    /// carried in every payload.
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
    /// `net` is the throughput this service derived from `src.net` and the
    /// previously stored counters — see `DataCache::update_one_vm`.
    pub fn update(&mut self, src: ContainerJsonModel, net: NetUsageJsonMode) {
        let started_at = src.started_at_or_none();
        self.cpu = src.cpu;
        self.mem = src.mem;
        self.files = src.files;
        self.net = net;
        self.net_prev = src.net.as_sample();
        self.disk = src.disk;
        self.labels = src.labels;
        self.enabled = src.enabled;
        self.image = src.image;
        self.instance = src.instance;
        // Adopt new started_at only when it's known; keep the previous value across a
        // stop or a transient inspect glitch. The wire type is an i64 where 0 means
        // "unknown", so serde hands us `Some(0)` and a bare `is_some()` would always
        // fire — overwriting a good timestamp with 1970-01-01 for every stopped
        // container, which is exactly what the stateless collector now sends.
        if let Some(started_at) = started_at {
            self.started_at = Some(started_at);
        }
    }
}

impl ContainerJsonModel {
    /// The collector encodes an unknown start time as `0`, not as an absent field, so
    /// serde produces `Some(0)`. Decode the sentinel here — the collector's own peer
    /// path does the same on its side of the wire.
    pub fn started_at_or_none(&self) -> Option<i64> {
        self.started_at.filter(|value| *value > 0)
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
    pub net: NetCountersJsonModel,
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
    /// Inbound throughput in MB/s. `None` until two collector samples exist.
    pub in_mbps: Option<f64>,
    /// Outbound throughput in MB/s.
    pub out_mbps: Option<f64>,
}

/// Raw cumulative network counters exactly as the collector read them from
/// Docker, plus the unix-millisecond instant of the reading.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct NetCountersJsonModel {
    pub rx_bytes: Option<i64>,
    pub tx_bytes: Option<i64>,
    #[serde(default)]
    pub sampled_at_unix_ms: i64,
}

impl NetCountersJsonModel {
    /// `Some` only when the reading is complete enough to be a rate anchor.
    pub fn as_sample(&self) -> Option<NetSample> {
        let rx = self.rx_bytes?;
        let tx = self.tx_bytes?;
        if self.sampled_at_unix_ms <= 0 {
            return None;
        }
        Some(NetSample {
            rx_bytes: rx,
            tx_bytes: tx,
            sampled_at_unix_ms: self.sampled_at_unix_ms,
        })
    }
}

/// One stored network reading, kept so the next poll can be turned into a rate.
#[derive(Clone, Debug, PartialEq)]
pub struct NetSample {
    pub rx_bytes: i64,
    pub tx_bytes: i64,
    pub sampled_at_unix_ms: i64,
}

impl NetSample {
    /// MB/s between this (older) sample and `next`. `None` when the pair can't
    /// produce a meaningful rate — no time elapsed, or the clock went backwards.
    pub fn rate_to(&self, next: &NetSample) -> Option<NetUsageJsonMode> {
        let elapsed_ms = next.sampled_at_unix_ms - self.sampled_at_unix_ms;
        if elapsed_ms <= 0 {
            return None;
        }

        const MB: f64 = 1024.0 * 1024.0;
        let secs = elapsed_ms as f64 / 1000.0;

        // Counters reset to a lower value on container restart — clamp
        // negatives to 0 instead of reporting a huge spike.
        Some(NetUsageJsonMode {
            in_mbps: Some((next.rx_bytes - self.rx_bytes).max(0) as f64 / secs / MB),
            out_mbps: Some((next.tx_bytes - self.tx_bytes).max(0) as f64 / secs / MB),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct DiskUsageJsonMode {
    /// Writable-layer size in bytes (the container's own data on top of the
    /// image). `None` until the api's slow disk-size rotation has measured it.
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
