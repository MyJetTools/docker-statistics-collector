mod get_status_action;
pub use get_status_action::*;
mod enable_action;
pub use enable_action::*;
mod disable_action;
pub use disable_action::*;

use std::sync::Arc;

use my_http_server::{
    macros::MyHttpInput, HttpContext, HttpFailResult, HttpOkResult, HttpOutput,
};

use crate::app::AppCtx;
use crate::http_client::ExecPermissionAction;

/// Shared by the status/enable/disable proxy actions.
#[derive(MyHttpInput)]
pub struct ExecPermissionInputModel {
    #[http_query(name = "env", description = "Environment name")]
    pub env: String,

    #[http_query(name = "url", description = "Master URL for the env")]
    pub url: String,

    #[http_query(name = "instance", description = "VM / instance name (ENV_INFO) owning the container")]
    pub instance: String,
}

pub async fn handle_exec_permission(
    app: &Arc<AppCtx>,
    input_data: ExecPermissionInputModel,
    ctx: &mut HttpContext,
    action: ExecPermissionAction,
) -> Result<HttpOkResult, HttpFailResult> {
    let user_id = crate::auth::user_from_http(ctx);

    let settings = app.settings_reader.get_settings().await;
    if !settings.is_env_allowed_for_user(&user_id, &input_data.env) {
        return Err(HttpFailResult::as_forbidden(Some(format!(
            "env '{}' is not accessible for user '{}'",
            input_data.env, user_id
        ))));
    }
    drop(settings);

    let fl_url = app
        .get_fl_url(input_data.env.as_str(), input_data.url.as_str())
        .await;

    let result =
        crate::http_client::exec_permission(fl_url, input_data.instance, user_id, action)
            .await
            .map_err(|err| HttpFailResult::as_fatal_error(err))?;

    HttpOutput::as_json(result).into_ok_result(false).into()
}
