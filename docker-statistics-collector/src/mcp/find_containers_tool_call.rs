use std::sync::Arc;

use mcp_server_middleware::*;
use my_ai_agent::macros::ApplyJsonSchema;
use serde::*;

use crate::app::{AppContext, ServiceInfo};

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct FindContainersInputData {
    #[property(
        description = "Substring to look for. Matched case-insensitively against container id, name, image, and labels (including com.docker.compose.service)."
    )]
    pub phrase: String,

    #[property(
        description = "If true, only running containers are returned. Defaults to true."
    )]
    pub only_running: Option<bool>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct FindContainersResponse {
    #[property(description = "Matched containers across this instance and all configured peers.")]
    pub containers: Vec<ContainerSummary>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ContainerSummary {
    #[property(description = "Container id.")]
    pub id: String,

    #[property(description = "ENV_INFO of the collector instance that owns this container.")]
    pub instance: String,

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

    #[property(description = "Latest CPU usage from the cache.")]
    pub cpu_usage: Option<f64>,

    #[property(description = "Latest memory usage in bytes.")]
    pub mem_usage: Option<i64>,

    #[property(description = "Memory limit in bytes.")]
    pub mem_limit: Option<i64>,

    #[property(description = "Container labels as key/value pairs.")]
    pub labels: Vec<LabelEntry>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct LabelEntry {
    #[property(description = "Label name")]
    pub label_key: String,

    #[property(description = "Label value")]
    pub label_value: String,
}

pub struct FindContainersHandler {
    app: Arc<AppContext>,
}

impl FindContainersHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for FindContainersHandler {
    const FUNC_NAME: &'static str = "find_containers";
    const DESCRIPTION: &'static str = "Search Docker containers by a phrase across this instance and all configured peers. Case-insensitive match across id, names, image, and labels. Each result carries an 'instance' field identifying its source.";
}

#[async_trait::async_trait]
impl McpToolCall<FindContainersInputData, FindContainersResponse> for FindContainersHandler {
    async fn execute_tool_call(
        &self,
        model: FindContainersInputData,
    ) -> Result<FindContainersResponse, String> {
        let needle = model.phrase.trim().to_lowercase();
        if needle.is_empty() {
            return Err("phrase must not be empty".to_string());
        }

        let only_running = model.only_running.unwrap_or(true);

        let local = self.app.cache.get_snapshot().await;
        let local_instance = self.app.get_env_info();

        let mut containers: Vec<ContainerSummary> = local
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
                containers.push(to_summary(c, peer.instance.clone()));
            }
        }

        Ok(FindContainersResponse { containers })
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
        .and_then(|l| l.get("com.docker.compose.service").cloned())
        .unwrap_or_default();

    let labels = c
        .labels
        .map(|m| {
            m.into_iter()
                .map(|(k, v)| LabelEntry {
                    label_key: k,
                    label_value: v,
                })
                .collect()
        })
        .unwrap_or_default();

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
        labels,
    }
}
