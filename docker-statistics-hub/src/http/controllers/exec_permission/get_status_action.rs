use std::sync::Arc;

use my_http_server::{macros::http_route, HttpContext, HttpFailResult, HttpOkResult};

use crate::app::AppCtx;
use crate::http_client::ExecPermissionAction;

use super::ExecPermissionInputModel;

#[http_route(
    method: "GET",
    route: "/api/exec-permission",
    controller: "ExecPermission",
    description: "Reports whether the exec_in_container MCP tool is unlocked on a VM, and for how long",
    summary: "Get exec permission status",
    input_data: ExecPermissionInputModel,
    result:[
        {status_code: 200, description: "Current exec permission window"},
    ]
)]
pub struct GetExecPermissionAction {
    app: Arc<AppCtx>,
}

impl GetExecPermissionAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetExecPermissionAction,
    input_data: ExecPermissionInputModel,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    super::handle_exec_permission(&action.app, input_data, ctx, ExecPermissionAction::Status).await
}
