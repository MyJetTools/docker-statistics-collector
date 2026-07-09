pub mod contracts;

mod get_status_action;
pub use get_status_action::*;
mod enable_action;
pub use enable_action::*;
mod disable_action;
pub use disable_action::*;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult, HttpOutput, HttpRequestHeaders};

use crate::app::AppContext;
use crate::peers_client::{
    route_exec_permission, ExecPermissionCommand, RouteExecPermissionResult,
};

use contracts::{ExecPermissionHttpInput, ExecPermissionHttpResponse};

/// Header the upstream reverse proxy injects to identify the authenticated user.
/// Mirrors `docker-statistics-api`'s `auth.rs` — we never validate it ourselves.
const SSL_USER_HEADER: &str = "x-ssl-user";

/// The three actions differ only by the command, so they all funnel through here.
pub async fn handle_exec_permission(
    app: &AppContext,
    input_data: ExecPermissionHttpInput,
    ctx: &HttpContext,
    command: ExecPermissionCommand,
) -> Result<HttpOkResult, HttpFailResult> {
    let by_user = resolve_user(ctx, input_data.by.as_deref());
    // A forwarded call must never bounce onward — only the instance it names may answer.
    let allow_forward = !input_data.no_forward.unwrap_or(false);

    match route_exec_permission(
        app,
        input_data.instance.as_deref(),
        command,
        by_user.as_str(),
        allow_forward,
    )
    .await
    {
        RouteExecPermissionResult::Ok(state) => HttpOutput::as_json(ExecPermissionHttpResponse {
            instance: state.instance,
            enabled: state.enabled,
            seconds_left: state.seconds_left,
        })
        .into_ok_result(false)
        .into(),
        RouteExecPermissionResult::InstanceNotFound => Err(HttpFailResult::as_not_found(
            match input_data.instance.as_deref() {
                Some(name) => format!("instance '{}' not found among this collector or its peers", name),
                None => "instance not found".to_string(),
            },
            false,
        )),
        RouteExecPermissionResult::PeerError(err) => Err(HttpFailResult::as_fatal_error(err)),
    }
}

/// Prefer the proxy-injected header; fall back to the `by` param a peer forwarded.
fn resolve_user(ctx: &HttpContext, by: Option<&str>) -> String {
    let from_header = ctx
        .request
        .get_headers()
        .try_get_case_insensitive_as_str(SSL_USER_HEADER)
        .ok()
        .flatten()
        .unwrap_or_default();

    if !from_header.is_empty() {
        return from_header.to_string();
    }

    by.unwrap_or_default().to_string()
}
