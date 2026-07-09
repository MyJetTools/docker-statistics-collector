use std::sync::Arc;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult};

use crate::app::AppContext;
use crate::peers_client::ExecPermissionCommand;

use super::contracts::{ExecPermissionHttpInput, ExecPermissionHttpResponse};

#[my_http_server::macros::http_route(
    method: "POST",
    route: "/api/exec-permission/disable",
    description: "Revoke the exec_in_container unlock on an instance immediately, without waiting for the window to expire",
    summary: "Disable exec now",
    controller: "ExecPermission",
    input_data: "ExecPermissionHttpInput",
    result:[
        {status_code: 200, description: "Exec locked again", model:"ExecPermissionHttpResponse" },
        {status_code: 404, description: "Instance not found" },
    ]
)]
pub struct DisableExecPermissionAction {
    app: Arc<AppContext>,
}

impl DisableExecPermissionAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &DisableExecPermissionAction,
    input_data: ExecPermissionHttpInput,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    super::handle_exec_permission(&action.app, input_data, ctx, ExecPermissionCommand::Disable).await
}
