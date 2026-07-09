use std::sync::Arc;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult};

use crate::app::AppContext;
use crate::peers_client::ExecPermissionCommand;

use super::contracts::{ExecPermissionHttpInput, ExecPermissionHttpResponse};

#[my_http_server::macros::http_route(
    method: "POST",
    route: "/api/exec-permission/enable",
    description: "Unlock the exec_in_container MCP tool on an instance for a limited time (exec_unlock_duration_secs, default 600). The window closes automatically; calling again extends it.",
    summary: "Enable exec for a limited time",
    controller: "ExecPermission",
    input_data: "ExecPermissionHttpInput",
    result:[
        {status_code: 200, description: "Exec unlocked; window returned", model:"ExecPermissionHttpResponse" },
        {status_code: 404, description: "Instance not found" },
    ]
)]
pub struct EnableExecPermissionAction {
    app: Arc<AppContext>,
}

impl EnableExecPermissionAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &EnableExecPermissionAction,
    input_data: ExecPermissionHttpInput,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    super::handle_exec_permission(&action.app, input_data, ctx, ExecPermissionCommand::Enable).await
}
