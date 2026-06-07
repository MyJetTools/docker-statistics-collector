use std::sync::Arc;

use mcp_server_middleware::*;
use my_ai_agent::macros::ApplyJsonSchema;
use serde::*;

use crate::app::{AppContext, ServiceInfo};

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ListExposedPortsInputData {
    #[property(
        description = "If true, only ports of currently running containers are listed. Defaults to false — stopped containers still reserve their host port mappings, so include them when picking a free port."
    )]
    pub only_running: Option<bool>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ListExposedPortsResponse {
    #[property(
        description = "One entry per instance (this collector and every peer) with all host-published ports, sorted ascending — use the gaps to choose the next free port."
    )]
    pub hosts: Vec<HostExposedPorts>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct HostExposedPorts {
    #[property(description = "Instance identifier (ENV_INFO of that collector).")]
    pub instance: String,

    #[property(description = "Distinct host ports already published on this instance, ascending.")]
    pub used_host_ports: Vec<u16>,

    #[property(description = "Detailed published-port mappings on this instance, sorted by host port.")]
    pub ports: Vec<ExposedPort>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ExposedPort {
    #[property(description = "Host (published) port — what is occupied on the host machine.")]
    pub host_port: u16,

    #[property(description = "Host IP the port is bound on, e.g. 0.0.0.0. Empty when not provided.")]
    pub host_ip: String,

    #[property(description = "Protocol: tcp, udp, or sctp.")]
    pub protocol: String,

    #[property(description = "Container-internal port the host port maps to.")]
    pub container_port: u16,

    #[property(description = "Container id owning this mapping.")]
    pub container_id: String,

    #[property(description = "Primary container name.")]
    pub container_name: String,

    #[property(description = "Image reference.")]
    pub image: String,

    #[property(description = "com.docker.compose.service label value, or empty string.")]
    pub compose_service: String,

    #[property(description = "True if the owning container is currently running.")]
    pub running: bool,
}

pub struct ListExposedPortsHandler {
    app: Arc<AppContext>,
}

impl ListExposedPortsHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for ListExposedPortsHandler {
    const FUNC_NAME: &'static str = "list_exposed_ports";
    const DESCRIPTION: &'static str = "List every host-published (exposed) port per instance (this collector and all peers), so you can see which ports are already taken on each VM and pick the next free one. Returns both a sorted list of used host ports and the detailed mapping (which container/service/protocol owns each).";
}

#[async_trait::async_trait]
impl McpToolCall<ListExposedPortsInputData, ListExposedPortsResponse> for ListExposedPortsHandler {
    async fn execute_tool_call(
        &self,
        model: ListExposedPortsInputData,
    ) -> Result<ListExposedPortsResponse, String> {
        let only_running = model.only_running.unwrap_or(false);

        let local = self.app.cache.get_snapshot().await;
        let local_instance = self.app.get_env_info();

        let mut hosts = vec![build_entry(&local_instance, &local, only_running)];

        for (peer_instance, peer_containers, _peer_hosts) in
            crate::peers_client::fanout_local_containers(&self.app).await
        {
            hosts.push(build_entry(&peer_instance, &peer_containers, only_running));
        }

        Ok(ListExposedPortsResponse { hosts })
    }
}

fn build_entry(instance: &str, containers: &[ServiceInfo], only_running: bool) -> HostExposedPorts {
    let mut ports: Vec<ExposedPort> = Vec::new();

    for c in containers {
        if only_running && !c.running {
            continue;
        }

        let compose_service = c
            .labels
            .as_ref()
            .and_then(|l| l.get("com.docker.compose.service").cloned())
            .unwrap_or_default();

        let container_name = c
            .names
            .first()
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_default();

        for p in &c.ports {
            let Some(host_port) = p.public_port else {
                continue; // not published to the host
            };
            ports.push(ExposedPort {
                host_port,
                host_ip: p.ip.clone().unwrap_or_default(),
                protocol: p.port_type.clone(),
                container_port: p.private_port,
                container_id: c.id.clone(),
                container_name: container_name.clone(),
                image: c.image.clone(),
                compose_service: compose_service.clone(),
                running: c.running,
            });
        }
    }

    ports.sort_by(|a, b| a.host_port.cmp(&b.host_port).then(a.protocol.cmp(&b.protocol)));

    let mut used_host_ports: Vec<u16> = ports.iter().map(|p| p.host_port).collect();
    used_host_ports.dedup();

    HostExposedPorts {
        instance: instance.to_string(),
        used_host_ports,
        ports,
    }
}
