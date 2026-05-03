use std::collections::BTreeMap;
use std::sync::Arc;

use mcp_server_middleware::*;
use my_ai_agent::macros::ApplyJsonSchema;
use serde::*;

use crate::app::{AppContext, ServiceInfo};

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ListServersAndServicesInputData {
    #[property(
        description = "If true, only running containers contribute to the service list. Defaults to true."
    )]
    pub only_running: Option<bool>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ListServersAndServicesResponse {
    #[property(description = "One entry per known instance (this collector and every configured peer).")]
    pub servers: Vec<ServerEntry>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ServerEntry {
    #[property(description = "Instance identifier (ENV_INFO of that collector).")]
    pub instance: String,

    #[property(description = "Distinct docker-compose services running on this instance with their published port mappings.")]
    pub services: Vec<ServiceEntry>,

    #[property(description = "Total number of containers reported by this instance after filtering.")]
    pub container_count: i64,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ServiceEntry {
    #[property(description = "com.docker.compose.service label value.")]
    pub service_name: String,

    #[property(description = "Distinct port mappings observed on containers of this service. Format: host_ip:host_port -> container_port (protocol).")]
    pub ports: Vec<PortMapping>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct PortMapping {
    #[property(description = "Host IP the port is bound on, e.g. 0.0.0.0. Empty string when not published to the host.")]
    pub host_ip: String,

    #[property(description = "Host (published) port. None means the port is not exposed outside the container network.")]
    pub host_port: Option<u16>,

    #[property(description = "Container-internal port the host port maps to.")]
    pub container_port: u16,

    #[property(description = "Protocol: tcp, udp, or sctp.")]
    pub protocol: String,
}

pub struct ListServersAndServicesHandler {
    app: Arc<AppContext>,
}

impl ListServersAndServicesHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for ListServersAndServicesHandler {
    const FUNC_NAME: &'static str = "list_servers_and_services";
    const DESCRIPTION: &'static str = "List every known collector instance (this server plus all configured peers) together with the distinct docker-compose service names running on each. Use to discover topology before drilling into find_containers/get_container_logs.";
}

#[async_trait::async_trait]
impl McpToolCall<ListServersAndServicesInputData, ListServersAndServicesResponse>
    for ListServersAndServicesHandler
{
    async fn execute_tool_call(
        &self,
        model: ListServersAndServicesInputData,
    ) -> Result<ListServersAndServicesResponse, String> {
        let only_running = model.only_running.unwrap_or(true);

        let local = self.app.cache.get_snapshot().await;
        let local_instance = self.app.get_env_info();

        let mut servers = vec![build_entry(&local_instance, &local, only_running)];

        for (peer_instance, peer_containers) in
            crate::peers_client::fanout_local_containers(&self.app).await
        {
            servers.push(build_entry(&peer_instance, &peer_containers, only_running));
        }

        Ok(ListServersAndServicesResponse { servers })
    }
}

fn build_entry(instance: &str, containers: &[ServiceInfo], only_running: bool) -> ServerEntry {
    // service_name -> set of unique port-mapping keys -> PortMapping
    let mut by_service: BTreeMap<String, BTreeMap<String, PortMapping>> = BTreeMap::new();
    let mut count: i64 = 0;

    for c in containers {
        if only_running && !c.running {
            continue;
        }
        count += 1;

        let service_name = c
            .labels
            .as_ref()
            .and_then(|l| l.get("com.docker.compose.service").cloned());

        let Some(service_name) = service_name else {
            continue;
        };

        let entry = by_service.entry(service_name).or_default();

        for p in &c.ports {
            let host_ip = p.ip.clone().unwrap_or_default();
            let key = format!(
                "{}:{}->{}/{}",
                host_ip,
                p.public_port.map(|n| n.to_string()).unwrap_or_default(),
                p.private_port,
                p.port_type
            );
            entry.entry(key).or_insert_with(|| PortMapping {
                host_ip,
                host_port: p.public_port,
                container_port: p.private_port,
                protocol: p.port_type.clone(),
            });
        }
    }

    let services = by_service
        .into_iter()
        .map(|(service_name, ports)| ServiceEntry {
            service_name,
            ports: ports.into_values().collect(),
        })
        .collect();

    ServerEntry {
        instance: instance.to_string(),
        services,
        container_count: count,
    }
}
