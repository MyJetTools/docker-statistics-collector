use std::sync::Arc;

use my_http_server::{
    macros::{http_route, MyHttpInput},
    HttpContext, HttpFailResult, HttpOkResult, HttpOutput,
};

use crate::app::AppCtx;

#[http_route(
    method: "POST",
    route: "/api/pass_phrase",
    controller: "Ssh",
    description: "Submits SSH private-key passphrase. Stored in process memory only.",
    summary: "Apply SSH passphrase",
    input_data: ApplyPassPhraseInputModel,
    result:[
        {status_code: 204, description: "Stored"},
    ]
)]
pub struct ApplyPassPhraseAction {
    app: Arc<AppCtx>,
}

impl ApplyPassPhraseAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

#[derive(MyHttpInput)]
pub struct ApplyPassPhraseInputModel {
    #[http_body(name = "pass_phrase", description = "SSH private-key passphrase")]
    pub pass_phrase: String,
}

async fn handle_request(
    action: &ApplyPassPhraseAction,
    input_data: ApplyPassPhraseInputModel,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    action
        .app
        .ssh_private_key_resolver
        .set_pass_phrase(input_data.pass_phrase)
        .await;

    HttpOutput::Empty.into_ok_result(false).into()
}
