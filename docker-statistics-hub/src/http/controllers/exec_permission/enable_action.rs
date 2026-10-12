use std::sync::Arc;

use my_http_server::{macros::http_route, HttpContext, HttpFailResult, HttpOkResult};

use crate::app::AppCtx;
use crate::http_client::ExecPermissionAction;

use super::ExecPermissionInputModel;

#[http_route(
    method: "POST",
    route: "/api/exec-permission/enable",
    controller: "ExecPermission",
    description: "Unlocks the exec_in_container MCP tool on a VM for a limited time. The window closes by itself; pressing again extends it.",
    summary: "Enable exec for a limited time",
    input_data: ExecPermissionInputModel,
    result:[
        {status_code: 200, description: "Exec unlocked; window returned"},
    ]
)]
pub struct EnableExecPermissionAction {
    app: Arc<AppCtx>,
}

impl EnableExecPermissionAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &EnableExecPermissionAction,
    input_data: ExecPermissionInputModel,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    super::handle_exec_permission(&action.app, input_data, ctx, ExecPermissionAction::Enable).await
}
