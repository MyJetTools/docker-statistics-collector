use std::sync::Arc;

use my_http_server::{macros::http_route, HttpContext, HttpFailResult, HttpOkResult};

use crate::app::AppCtx;
use crate::http_client::ExecPermissionAction;

use super::ExecPermissionInputModel;

#[http_route(
    method: "POST",
    route: "/api/exec-permission/disable",
    controller: "ExecPermission",
    description: "Revokes the exec_in_container unlock on a VM immediately, without waiting for the window to expire",
    summary: "Disable exec now",
    input_data: ExecPermissionInputModel,
    result:[
        {status_code: 200, description: "Exec locked again"},
    ]
)]
pub struct DisableExecPermissionAction {
    app: Arc<AppCtx>,
}

impl DisableExecPermissionAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &DisableExecPermissionAction,
    input_data: ExecPermissionInputModel,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    super::handle_exec_permission(&action.app, input_data, ctx, ExecPermissionAction::Disable).await
}
