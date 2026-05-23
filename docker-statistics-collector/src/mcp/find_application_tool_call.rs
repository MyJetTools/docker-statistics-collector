use std::sync::Arc;

use mcp_server_middleware::*;
use my_ai_agent::macros::ApplyJsonSchema;
use regex::Regex;
use serde::*;

use crate::app::{AppContext, ServiceInfo};

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct FindApplicationInputData {
    #[property(
        description = "Regular expression matched against application identity (compose service name, container names, and image). Case-insensitive. Standard Rust regex syntax."
    )]
    pub pattern: String,

    #[property(
        description = "If true, only running containers are returned. Defaults to true."
    )]
    pub only_running: Option<bool>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct FindApplicationResponse {
    #[property(description = "Matched applications across this server and all reachable hosts.")]
    pub applications: Vec<ApplicationMatch>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ApplicationMatch {
    #[property(description = "Source host identifier (ENV_INFO of that collector).")]
    pub instance: String,

    #[property(description = "Container id.")]
    pub id: String,

    #[property(description = "Container names reported by Docker.")]
    pub names: Vec<String>,

    #[property(description = "Image reference.")]
    pub image: String,

    #[property(description = "Docker state, e.g. running, exited.")]
    pub state: String,

    #[property(description = "Human-readable status string.")]
    pub status: String,

    #[property(description = "True if the container is currently running.")]
    pub running: bool,

    #[property(description = "Value of the com.docker.compose.service label, or empty string if absent.")]
    pub compose_service: String,

    #[property(description = "File descriptors currently open by the container's main process. None when the host /proc is not reachable.")]
    pub open_files: Option<i64>,

    #[property(description = "nofile soft limit (RLIMIT_NOFILE) of the container's main process. None when the host /proc is not reachable.")]
    pub fd_limit: Option<i64>,

    #[property(description = "Which fields the regex matched: compose_service, name, and/or image.")]
    pub matched_on: Vec<String>,
}

pub struct FindApplicationHandler {
    app: Arc<AppContext>,
}

impl FindApplicationHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for FindApplicationHandler {
    const FUNC_NAME: &'static str = "find_application";
    const DESCRIPTION: &'static str = "Search for an application by regular expression matched against the docker-compose service name, container names, and image. Returns the matching containers' description and the host (instance) each one runs on.";
}

#[async_trait::async_trait]
impl McpToolCall<FindApplicationInputData, FindApplicationResponse> for FindApplicationHandler {
    async fn execute_tool_call(
        &self,
        model: FindApplicationInputData,
    ) -> Result<FindApplicationResponse, String> {
        let pattern = model.pattern.trim();
        if pattern.is_empty() {
            return Err("pattern must not be empty".to_string());
        }

        let regex = Regex::new(&format!("(?i){}", pattern))
            .map_err(|e| format!("invalid regex: {}", e))?;

        let only_running = model.only_running.unwrap_or(true);

        let mut applications = Vec::new();

        let local = self.app.cache.get_snapshot().await;
        let local_instance = self.app.get_env_info();
        for c in local.into_iter().filter(|c| !only_running || c.running) {
            if let Some(m) = match_application(&c, &regex, &local_instance) {
                applications.push(m);
            }
        }

        for (peer_instance, peer_containers, _peer_hosts) in
            crate::peers_client::fanout_local_containers(&self.app).await
        {
            for c in peer_containers
                .into_iter()
                .filter(|c| !only_running || c.running)
            {
                if let Some(m) = match_application(&c, &regex, &peer_instance) {
                    applications.push(m);
                }
            }
        }

        Ok(FindApplicationResponse { applications })
    }
}

fn match_application(c: &ServiceInfo, regex: &Regex, instance: &str) -> Option<ApplicationMatch> {
    let compose_service = c
        .labels
        .as_ref()
        .and_then(|l| l.get("com.docker.compose.service").cloned())
        .unwrap_or_default();

    let mut matched_on = Vec::new();

    if !compose_service.is_empty() && regex.is_match(&compose_service) {
        matched_on.push("compose_service".to_string());
    }
    if c.names.iter().any(|n| regex.is_match(n)) {
        matched_on.push("name".to_string());
    }
    if regex.is_match(&c.image) {
        matched_on.push("image".to_string());
    }

    if matched_on.is_empty() {
        return None;
    }

    Some(ApplicationMatch {
        instance: instance.to_string(),
        id: c.id.clone(),
        names: c.names.clone(),
        image: c.image.clone(),
        state: c.state.clone(),
        status: c.status.clone(),
        running: c.running,
        compose_service,
        open_files: c.open_files,
        fd_limit: c.fd_limit,
        matched_on,
    })
}
