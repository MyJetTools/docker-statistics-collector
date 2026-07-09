use std::sync::Arc;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult};

use crate::app::AppContext;
use crate::peers_client::ExecPermissionCommand;

use super::contracts::{ExecPermissionHttpInput, ExecPermissionHttpResponse};

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/api/exec-permission",
    description: "Report whether the exec_in_container MCP tool is currently unlocked on an instance, and for how much longer",
    summary: "Get exec permission status",
    controller: "ExecPermission",
    input_data: "ExecPermissionHttpInput",
    result:[
        {status_code: 200, description: "Current exec permission window", model:"ExecPermissionHttpResponse" },
        {status_code: 404, description: "Instance not found" },
    ]
)]
pub struct GetExecPermissionAction {
    app: Arc<AppContext>,
}

impl GetExecPermissionAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetExecPermissionAction,
    input_data: ExecPermissionHttpInput,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    super::handle_exec_permission(&action.app, input_data, ctx, ExecPermissionCommand::Status).await
}
