use std::collections::HashMap;
use std::sync::Arc;

use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, InitializeRequestParams, InitializeResult, ProtocolVersion,
        ServerCapabilities, ServerInfo,
    },
    schemars,
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};

use crate::app::{AppContext, ServiceInfo};
use crate::http::controllers::containers_controller::{route_logs, RouteLogsResult};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Search containers by a free-form phrase.")]
pub struct FindContainersParams {
    #[schemars(
        description = "Substring to look for. Matched case-insensitively against container id, name, image, and labels (including com.docker.compose.service)."
    )]
    pub phrase: String,
    #[schemars(description = "If true, only running containers are returned. Defaults to true.")]
    pub only_running: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Fetch tail logs of a single container.")]
pub struct GetLogsParams {
    #[schemars(description = "Container id (full or prefix as returned by find_containers).")]
    pub container_id: String,
    #[schemars(description = "Number of trailing log lines to return. Defaults to 200.")]
    pub tail: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ContainerSummary {
    id: String,
    instance: String,
    names: Vec<String>,
    image: String,
    state: String,
    status: String,
    running: bool,
    compose_service: Option<String>,
    cpu_usage: Option<f64>,
    mem_usage: Option<i64>,
    mem_limit: Option<i64>,
    ports: Vec<PortSummary>,
    labels: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
struct PortSummary {
    ip: Option<String>,
    private_port: u16,
    public_port: Option<u16>,
    port_type: String,
}

#[derive(Clone)]
pub struct DockerMcpServer {
    app: Arc<AppContext>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl DockerMcpServer {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self {
            app,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl DockerMcpServer {
    #[tool(
        description = "Search Docker containers by a phrase across this instance and all configured peers. Returns a JSON array with id, instance, names, image, state, ports, labels, the compose service name, and the latest CPU/memory snapshot. Match is case-insensitive across id, names, image, and labels."
    )]
    async fn find_containers(
        &self,
        Parameters(FindContainersParams {
            phrase,
            only_running,
        }): Parameters<FindContainersParams>,
    ) -> Result<CallToolResult, McpError> {
        let only_running = only_running.unwrap_or(true);
        let needle = phrase.trim().to_lowercase();

        if needle.is_empty() {
            return Err(McpError::invalid_params(
                "phrase must not be empty".to_string(),
                None,
            ));
        }

        let local = self.app.cache.get_snapshot().await;
        let local_instance = self.app.get_env_info();

        let mut matches: Vec<ContainerSummary> = local
            .into_iter()
            .filter(|c| !only_running || c.running)
            .filter(|c| matches_phrase(c, &needle))
            .map(|c| to_summary(c, local_instance.clone()))
            .collect();

        for peer in self.app.peers_cache.get_snapshot().await {
            for c in peer
                .containers
                .into_iter()
                .filter(|c| !only_running || c.running)
                .filter(|c| matches_phrase(c, &needle))
            {
                matches.push(to_summary(c, peer.instance.clone()));
            }
        }

        let payload = serde_json::to_string_pretty(&matches).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize result: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(payload)]))
    }

    #[tool(
        description = "Fetch the tail of a container's combined stdout/stderr logs. Use container_id from find_containers; works for containers on this instance and on any configured peer (auto-routed)."
    )]
    async fn get_container_logs(
        &self,
        Parameters(GetLogsParams { container_id, tail }): Parameters<GetLogsParams>,
    ) -> Result<CallToolResult, McpError> {
        let id = container_id.trim();
        if id.is_empty() {
            return Err(McpError::invalid_params(
                "container_id must not be empty".to_string(),
                None,
            ));
        }

        let tail = tail.unwrap_or(200);

        match route_logs(&self.app, id, tail).await {
            RouteLogsResult::Ok(bytes) => {
                let text = sanitize_log_stream(&bytes);
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            RouteLogsResult::NotFound => Err(McpError::invalid_params(
                format!(
                    "container {} not found on this instance or any peer",
                    id
                ),
                None,
            )),
            RouteLogsResult::PeerError(err) => Err(McpError::internal_error(err, None)),
        }
    }
}

#[tool_handler]
impl ServerHandler for DockerMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2025_06_18)
            .with_instructions(
                "Tools:\n\
                 - find_containers(phrase, only_running?): list containers matching a phrase \
                 across this instance and all configured peers. Each result carries an \
                 'instance' field identifying its source.\n\
                 - get_container_logs(container_id, tail?): fetch tail logs by container id. \
                 Auto-routes to the correct peer when the container lives on another instance.\n\
                 Typical flow: call find_containers first to resolve the id, then \
                 get_container_logs for diagnostics.",
            )
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }
}

fn matches_phrase(c: &ServiceInfo, needle: &str) -> bool {
    if c.id.to_lowercase().contains(needle) {
        return true;
    }
    if c.image.to_lowercase().contains(needle) {
        return true;
    }
    if c.names.iter().any(|n| n.to_lowercase().contains(needle)) {
        return true;
    }
    if let Some(labels) = c.labels.as_ref() {
        for (k, v) in labels {
            if k.to_lowercase().contains(needle) || v.to_lowercase().contains(needle) {
                return true;
            }
        }
    }
    false
}

fn to_summary(c: ServiceInfo, instance: String) -> ContainerSummary {
    let compose_service = c
        .labels
        .as_ref()
        .and_then(|l| l.get("com.docker.compose.service").cloned());

    let ports = c
        .ports
        .into_iter()
        .map(|p| PortSummary {
            ip: p.ip,
            private_port: p.private_port,
            public_port: p.public_port,
            port_type: p.port_type,
        })
        .collect();

    ContainerSummary {
        id: c.id,
        instance,
        names: c.names,
        image: c.image,
        state: c.state,
        status: c.status,
        running: c.running,
        compose_service,
        cpu_usage: c.cpu_usage,
        mem_usage: c.mem_usage,
        mem_limit: c.mem_limit,
        ports,
        labels: c.labels,
    }
}

// Docker's logs endpoint returns a multiplexed stream when no TTY is attached:
// each frame is prefixed with an 8-byte header (stream type + 4 zero bytes + 4-byte BE length).
// Strip those headers so the model sees plain text. If the stream is not multiplexed
// (TTY mode), fall back to a lossy UTF-8 decode of the raw bytes.
fn sanitize_log_stream(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        let stream_type = bytes[i];
        if stream_type > 2 || bytes[i + 1] != 0 || bytes[i + 2] != 0 || bytes[i + 3] != 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        let len = u32::from_be_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]])
            as usize;
        let start = i + 8;
        let end = start + len;
        if end > bytes.len() {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        out.push_str(&String::from_utf8_lossy(&bytes[start..end]));
        i = end;
    }
    if i != bytes.len() {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    out
}
