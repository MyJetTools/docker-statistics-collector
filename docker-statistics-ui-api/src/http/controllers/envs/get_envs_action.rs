use std::sync::Arc;

use my_http_server::{macros::http_route, HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use crate::app::AppCtx;
use crate::models::EnvsHttpModel;

#[http_route(
    method: "GET",
    route: "/api/envs",
    controller: "Envs",
    description: "Lists configured environments and indicates whether the server still needs the SSH passphrase",
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
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let settings = action.app.settings_reader.get_settings().await;

    let envs = settings.get_envs();

    let mut request_pass_key = false;
    if settings.prompt_pass_phrase.unwrap_or(false)
        && !action.app.ssh_private_key_resolver.private_key_is_loaded()
    {
        request_pass_key = true;
    }

    let response = EnvsHttpModel {
        envs,
        request_pass_key,
    };

    HttpOutput::as_json(response).into_ok_result(false).into()
}
