use serde::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VmModel {
    pub api_url: String,
    pub cpu: f64,
    pub mem: i64,
    pub mem_limit: i64,
    pub containers_amount: usize,
    // Total file descriptors open by the VM's containers.
    pub open_files: i64,
    /// Sum of inbound network throughput (MB/s) across the VM's containers.
    #[serde(default)]
    pub net_in_mbps: f64,
    /// Sum of outbound network throughput (MB/s) across the VM's containers.
    #[serde(default)]
    pub net_out_mbps: f64,
    /// Host physical memory in bytes — reported by the collector reading
    /// `/proc/meminfo` on the peer's host. `None` when `/proc` is not
    /// bind-mounted into the collector container or the platform has no `/proc`.
    #[serde(default)]
    pub host_mem_total: Option<i64>,
    #[serde(default)]
    pub host_mem_available: Option<i64>,
    #[serde(default)]
    pub host_mem_used: Option<i64>,
    /// Logical CPU count of the host VM. `None` when unknown.
    #[serde(default)]
    pub host_cpu_count: Option<u32>,
    /// Host physical disks. `None` for the synthetic "All VMs" aggregate (the
    /// summary card hides disks); empty `Some(vec)` when the host root
    /// filesystem isn't mounted into the collector.
    #[serde(default)]
    pub host_disks: Option<Vec<DiskModel>>,
}

/// One physical filesystem on the host (matches the API's `DiskModel`). Sizes
/// are bytes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl DiskModel {
    /// Percent of the filesystem in use, 0..100. Defined once, here, so the VM
    /// rail and the Host disks board cannot show the same disk at two fill levels.
    pub fn used_pct(&self) -> f64 {
        if self.total <= 0 {
            return 0.0;
        }
        (self.used as f64 / self.total as f64 * 100.0).clamp(0.0, 100.0)
    }
}
