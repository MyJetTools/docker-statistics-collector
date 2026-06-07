use std::sync::Arc;

use mcp_server_middleware::*;
use my_ai_agent::macros::ApplyJsonSchema;
use serde::*;

use crate::app::AppContext;
use crate::peers_client::{fanout_exec, RouteExecResult};

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ExecInContainerInputData {
    #[property(description = "Container id (full or prefix as returned by find_containers).")]
    pub container_id: String,

    #[property(
        description = "Shell command to run inside the container. Executed as: sh -c \"<command>\". E.g. `ls -la /app`, `cat /etc/hostname`, `ps aux`."
    )]
    pub command: String,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ExecInContainerResponse {
    #[property(description = "Combined stdout/stderr of the command.")]
    pub output: String,

    #[property(description = "Process exit code. 0 means success. null if it couldn't be read.")]
    pub exit_code: Option<i64>,
}

pub struct ExecInContainerHandler {
    app: Arc<AppContext>,
}

impl ExecInContainerHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for ExecInContainerHandler {
    const FUNC_NAME: &'static str = "exec_in_container";
    const DESCRIPTION: &'static str = "Run a shell command inside a container (like `docker exec`, as `sh -c \"<command>\"`) and return its combined stdout/stderr and exit code. Use container_id from find_containers; works for containers on this instance and on any configured peer (auto-routed). Powerful — runs arbitrary commands in the container.";
}

#[async_trait::async_trait]
impl McpToolCall<ExecInContainerInputData, ExecInContainerResponse> for ExecInContainerHandler {
    async fn execute_tool_call(
        &self,
        model: ExecInContainerInputData,
    ) -> Result<ExecInContainerResponse, String> {
        let id = model.container_id.trim();
        if id.is_empty() {
            return Err("container_id must not be empty".to_string());
        }
        let command = model.command.trim();
        if command.is_empty() {
            return Err("command must not be empty".to_string());
        }

        match fanout_exec(&self.app, id, command).await {
            RouteExecResult::Ok(result) => Ok(ExecInContainerResponse {
                output: result.output,
                exit_code: result.exit_code,
            }),
            RouteExecResult::NotFound => Err(format!(
                "container {} not found on this instance or any peer",
                id
            )),
            RouteExecResult::PeerError(err) => Err(err),
        }
    }
}
