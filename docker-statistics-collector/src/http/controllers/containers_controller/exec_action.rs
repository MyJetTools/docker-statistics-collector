use crate::app::AppContext;
use crate::peers_client::{fanout_exec, ExecOrigin, RouteExecResult};

use my_http_server::{macros::MyHttpInput, HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use std::sync::Arc;

use super::contracts::ContainerExecHttpResponse;

#[my_http_server::macros::http_route(
    method: "POST",
    route: "/api/containers/exec",
    description: "Run a command inside a container (sh -c). Auto-routed to the owning instance or peer.",
    summary: "Exec a command in a container",
    controller: "Containers",
    input_data: "ExecHttpInput",
    result:[
        {status_code: 200, description: "Command output", model:"ContainerExecHttpResponse" },
    ]
)]
pub struct ExecAction {
    app: Arc<AppContext>,
}

impl ExecAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &ExecAction,
    input_data: ExecHttpInput,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let command = input_data.command.trim();
    if command.is_empty() {
        return Err(HttpFailResult::as_validation_error(
            "command must not be empty".to_string(),
        ));
    }

    let origin = ExecOrigin::from_mcp_flag(input_data.mcp);

    match fanout_exec(&action.app, input_data.id.as_str(), command, origin).await {
        RouteExecResult::Ok(result) => HttpOutput::as_json(ContainerExecHttpResponse {
            container_id: input_data.id,
            output: result.output,
            exit_code: result.exit_code,
        })
        .into_ok_result(false)
        .into(),
        RouteExecResult::NotFound => Err(HttpFailResult::as_not_found(
            format!(
                "container {} not found on this instance or any peer",
                input_data.id
            ),
            false,
        )),
        RouteExecResult::Forbidden(msg) => Err(HttpFailResult::as_forbidden(Some(msg))),
        RouteExecResult::PeerError(err) => Err(HttpFailResult::as_fatal_error(err)),
    }
}

#[derive(MyHttpInput)]
pub struct ExecHttpInput {
    #[http_query(description:"Container id")]
    pub id: String,
    #[http_query(description:"Command to run, executed as: sh -c \"<command>\"")]
    pub command: String,
    #[http_query(description:"Internal: set by a peer collector when the call originated from the gated MCP tool")]
    pub mcp: Option<bool>,
}
