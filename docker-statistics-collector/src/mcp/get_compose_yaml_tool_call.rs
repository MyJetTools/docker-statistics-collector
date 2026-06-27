use std::io::Read;
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use flate2::read::GzDecoder;
use mcp_server_middleware::*;
use my_ai_agent::macros::ApplyJsonSchema;
use serde::*;

use crate::app::{AppContext, ServiceInfo};

/// Docker label release-mcp stamps onto a container, holding the
/// gzip+base64-encoded docker-compose.yaml that produced it.
const COMPOSE_LABEL: &str = "com.release-mcp.compose-yaml";

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetComposeYamlInputData {
    #[property(description = "Container id (full or prefix as returned by find_containers).")]
    pub container_id: String,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetComposeYamlResponse {
    #[property(description = "ENV_INFO of the collector instance that owns the container.")]
    pub instance: String,

    #[property(description = "Full id of the matched container.")]
    pub container_id: String,

    #[property(description = "Primary container name, without the leading slash.")]
    pub container_name: String,

    #[property(description = "Decoded docker-compose.yaml from the com.release-mcp.compose-yaml label.")]
    pub compose_yaml: String,
}

pub struct GetComposeYamlHandler {
    app: Arc<AppContext>,
}

impl GetComposeYamlHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for GetComposeYamlHandler {
    const FUNC_NAME: &'static str = "get_compose_yaml";
    const DESCRIPTION: &'static str = "Read and decode the docker-compose.yaml that produced a container, stored gzip+base64-encoded in its com.release-mcp.compose-yaml label. Use container_id from find_containers; works for containers on this instance and on any configured peer (auto-routed). Errors if the container carries no such label.";
}

#[async_trait::async_trait]
impl McpToolCall<GetComposeYamlInputData, GetComposeYamlResponse> for GetComposeYamlHandler {
    async fn execute_tool_call(
        &self,
        model: GetComposeYamlInputData,
    ) -> Result<GetComposeYamlResponse, String> {
        let id = model.container_id.trim();
        if id.is_empty() {
            return Err("container_id must not be empty".to_string());
        }

        let local_instance = self.app.get_env_info();
        let local = self.app.cache.get_snapshot().await;

        // Labels (including the compose blob) already live in the cached
        // snapshot, so resolving the container locally — then across peers —
        // is enough; no extra Docker/peer round-trip is needed.
        let mut found = local
            .into_iter()
            .find(|c| matches_id(c, id))
            .map(|c| (local_instance, c));

        if found.is_none() {
            for (peer_instance, peer_containers, _peer_hosts) in
                crate::peers_client::fanout_local_containers(&self.app).await
            {
                if let Some(c) = peer_containers.into_iter().find(|c| matches_id(c, id)) {
                    found = Some((peer_instance, c));
                    break;
                }
            }
        }

        let Some((instance, container)) = found else {
            return Err(format!(
                "container {} not found on this instance or any peer",
                id
            ));
        };

        let raw = container
            .labels
            .as_ref()
            .and_then(|l| l.get(COMPOSE_LABEL))
            .filter(|v| !v.trim().is_empty());

        let Some(raw) = raw else {
            return Err(format!(
                "container {} has no '{}' label",
                container.id, COMPOSE_LABEL
            ));
        };

        let compose_yaml = decode_compose_label(raw);

        let container_name = container
            .names
            .first()
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_default();

        Ok(GetComposeYamlResponse {
            instance,
            container_id: container.id,
            container_name,
            compose_yaml,
        })
    }
}

fn matches_id(c: &ServiceInfo, id: &str) -> bool {
    c.id == id || c.id.starts_with(id)
}

/// base64 → gzip → text. If anything fails to inflate we fall back to the raw
/// bytes (uncompressed label, foreign format, …) so the content stays readable
/// instead of being lost — same logic as release-mcp's compose_blob.rs.
fn decode_compose_label(value: &str) -> String {
    if let Ok(bytes) = BASE64.decode(value.trim()) {
        let mut out = String::new();
        if GzDecoder::new(&bytes[..]).read_to_string(&mut out).is_ok() {
            return out; // inflated YAML
        }
        if let Ok(text) = String::from_utf8(bytes) {
            return text; // base64 of raw text
        }
    }
    value.to_string() // not base64 → already raw text
}
