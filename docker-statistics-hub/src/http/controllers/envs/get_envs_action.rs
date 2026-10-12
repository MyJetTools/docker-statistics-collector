use std::sync::Arc;

use my_http_server::{macros::http_route, HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use crate::app::AppCtx;
use crate::models::EnvsHttpModel;

#[http_route(
    method: "GET",
    route: "/api/envs",
    controller: "Envs",
    description: "Lists the environments visible to the calling user, and who the caller is",
    summary: "List environments",
    result:[
        {status_code: 200, description: "List of envs + pass-key flag"},
    ]
)]
pub struct GetEnvsAction {
    app: Arc<AppCtx>,
}

impl GetEnvsAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetEnvsAction,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let settings = action.app.settings_reader.get_settings().await;

    let user_id = crate::auth::user_from_http(ctx);
    let envs = settings.get_envs_for_user(&user_id);

    let response = EnvsHttpModel { envs, user_id };

    HttpOutput::as_json(response)
        .with_compression(1024)
        .into_ok_result(false)
        .into()
}
