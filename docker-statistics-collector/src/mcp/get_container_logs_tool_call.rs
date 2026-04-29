use std::sync::Arc;

use mcp_server_middleware::*;
use my_ai_agent::macros::ApplyJsonSchema;
use serde::*;

use crate::app::AppContext;
use crate::http::controllers::containers_controller::{route_logs, RouteLogsResult};

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetContainerLogsInputData {
    #[property(description = "Container id (full or prefix as returned by find_containers).")]
    pub container_id: String,

    #[property(description = "Number of trailing log lines to return. Defaults to 200.")]
    pub tail: Option<u32>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetContainerLogsResponse {
    #[property(description = "Combined stdout/stderr tail with Docker's multiplexed framing stripped.")]
    pub logs: String,
}

pub struct GetContainerLogsHandler {
    app: Arc<AppContext>,
}

impl GetContainerLogsHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for GetContainerLogsHandler {
    const FUNC_NAME: &'static str = "get_container_logs";
    const DESCRIPTION: &'static str = "Fetch the tail of a container's combined stdout/stderr logs. Use container_id from find_containers; works for containers on this instance and on any configured peer (auto-routed).";
}

#[async_trait::async_trait]
impl McpToolCall<GetContainerLogsInputData, GetContainerLogsResponse> for GetContainerLogsHandler {
    async fn execute_tool_call(
        &self,
        model: GetContainerLogsInputData,
    ) -> Result<GetContainerLogsResponse, String> {
        let id = model.container_id.trim();
        if id.is_empty() {
            return Err("container_id must not be empty".to_string());
        }

        let tail = model.tail.unwrap_or(200);

        match route_logs(&self.app, id, tail).await {
            RouteLogsResult::Ok(bytes) => Ok(GetContainerLogsResponse {
                logs: sanitize_log_stream(&bytes),
            }),
            RouteLogsResult::NotFound => Err(format!(
                "container {} not found on this instance or any peer",
                id
            )),
            RouteLogsResult::PeerError(err) => Err(err),
        }
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
