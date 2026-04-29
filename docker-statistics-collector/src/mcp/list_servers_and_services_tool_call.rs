use std::collections::BTreeSet;
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

    #[property(description = "Distinct values of the com.docker.compose.service label, sorted.")]
    pub services: Vec<String>,

    #[property(description = "Total number of containers reported by this instance after filtering.")]
    pub container_count: i64,
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

        for peer in self.app.peers_cache.get_snapshot().await {
            servers.push(build_entry(&peer.instance, &peer.containers, only_running));
        }

        Ok(ListServersAndServicesResponse { servers })
    }
}

fn build_entry(instance: &str, containers: &[ServiceInfo], only_running: bool) -> ServerEntry {
    let mut services: BTreeSet<String> = BTreeSet::new();
    let mut count: i64 = 0;

    for c in containers {
        if only_running && !c.running {
            continue;
        }
        count += 1;

        if let Some(labels) = c.labels.as_ref() {
            if let Some(svc) = labels.get("com.docker.compose.service") {
                services.insert(svc.clone());
            }
        }
    }

    ServerEntry {
        instance: instance.to_string(),
        services: services.into_iter().collect(),
        container_count: count,
    }
}
