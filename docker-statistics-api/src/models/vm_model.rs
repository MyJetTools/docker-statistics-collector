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
    /// Host physical disks. `None` for the synthetic "All VMs" aggregate;
    /// empty `Some(vec)` when the host root filesystem is not mounted.
    #[serde(default)]
    pub host_disks: Option<Vec<crate::models::DiskModel>>,
}
