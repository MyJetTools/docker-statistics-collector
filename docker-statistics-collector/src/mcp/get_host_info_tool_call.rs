use std::sync::Arc;

use mcp_server_middleware::*;
use my_ai_agent::macros::ApplyJsonSchema;
use serde::*;

use crate::app::AppContext;
use crate::http::controllers::containers_controller::contracts::HostMemEntryHttpModel;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetHostInfoInputData {}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetHostInfoResponse {
    #[property(
        description = "One entry per known instance (this collector and every configured peer) describing the physical host machine: memory, CPU count and physical disks."
    )]
    pub hosts: Vec<HostEntry>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct HostEntry {
    #[property(description = "Instance identifier (ENV_INFO of that collector).")]
    pub instance: String,

    #[property(description = "Total physical RAM of the host in bytes.")]
    pub mem_total: i64,

    #[property(description = "Available RAM of the host in bytes.")]
    pub mem_available: i64,

    #[property(description = "Used RAM of the host in bytes.")]
    pub mem_used: i64,

    #[property(description = "Logical CPU count of the host. 0 means unknown.")]
    pub cpu_count: i32,

    #[property(
        description = "Physical disks on the host. Empty when the host root filesystem is not bind-mounted into the collector container (-v /:/host/root:ro)."
    )]
    pub disks: Vec<DiskEntry>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct DiskEntry {
    #[property(description = "Block device, e.g. /dev/sda1.")]
    pub device: String,

    #[property(description = "Mount point on the host, e.g. / or /data.")]
    pub mount_point: String,

    #[property(description = "Filesystem type, e.g. ext4, xfs, btrfs.")]
    pub fs_type: String,

    #[property(description = "Total size of the filesystem in bytes.")]
    pub total: i64,

    #[property(description = "Used space in bytes.")]
    pub used: i64,

    #[property(description = "Space available to unprivileged users in bytes.")]
    pub available: i64,
}

impl From<HostMemEntryHttpModel> for HostEntry {
    fn from(h: HostMemEntryHttpModel) -> Self {
        Self {
            instance: h.instance,
            mem_total: h.total,
            mem_available: h.available,
            mem_used: h.used,
            cpu_count: h.cpu_count,
            disks: h
                .disks
                .into_iter()
                .map(|d| DiskEntry {
                    device: d.device,
                    mount_point: d.mount_point,
                    fs_type: d.fs_type,
                    total: d.total,
                    used: d.used,
                    available: d.available,
                })
                .collect(),
        }
    }
}

pub struct GetHostInfoHandler {
    app: Arc<AppContext>,
}

impl GetHostInfoHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for GetHostInfoHandler {
    const FUNC_NAME: &'static str = "get_host_info";
    const DESCRIPTION: &'static str = "Get host-level machine stats (NOT per-container) for this collector and every configured peer: physical RAM, logical CPU count and physical disks (device, mount point, filesystem type, total/used/available bytes). Use to see how much disk space is left on each host.";
}

#[async_trait::async_trait]
impl McpToolCall<GetHostInfoInputData, GetHostInfoResponse> for GetHostInfoHandler {
    async fn execute_tool_call(
        &self,
        _model: GetHostInfoInputData,
    ) -> Result<GetHostInfoResponse, String> {
        let instance = self.app.get_env_info();
        let proc_base = self.app.settings_model.host_proc_path().to_string();
        let root_base = self.app.settings_model.host_root_path().to_string();
        let ignore_disks = self.app.settings_model.ignore_disks().to_vec();

        // Local host — memory + physical disks (same source as /api/containers).
        let local = tokio::task::spawn_blocking(move || {
            let mem = crate::host_mem::read(&proc_base);
            let disks = crate::host_disks::read(&proc_base, &root_base, &ignore_disks);
            (mem, disks)
        })
        .await
        .map_err(|err| format!("host read join error: {:?}", err))?;

        let mut raw: Vec<HostMemEntryHttpModel> = Vec::new();
        if let (Some(snap), disks) = local {
            raw.push(HostMemEntryHttpModel::from_snapshot(instance, snap, disks));
        }

        // Peers — their already-collected host entries (memory + disks).
        for (_peer_instance, _peer_containers, peer_hosts) in
            crate::peers_client::fanout_local_containers(&self.app).await
        {
            raw.extend(peer_hosts);
        }

        let hosts = raw.into_iter().map(HostEntry::from).collect();

        Ok(GetHostInfoResponse { hosts })
    }
}
