use std::sync::Arc;

use my_http_server::{
    macros::{http_route, MyHttpInput},
    HttpContext, HttpFailResult, HttpOkResult, HttpOutput,
};

use crate::app::AppCtx;

#[http_route(
    method: "GET",
    route: "/api/processes",
    controller: "Processes",
    description: "Proxies container processes from the env's master collector",
    summary: "Read container processes",
    input_data: GetProcessesInputModel,
    result:[
        {status_code: 200, description: "Array of ProcessHttpModel"},
    ]
)]
pub struct GetProcessesAction {
    app: Arc<AppCtx>,
}

impl GetProcessesAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

#[derive(MyHttpInput)]
pub struct GetProcessesInputModel {
    #[http_query(name = "env", description = "Environment name")]
    pub env: String,

    #[http_query(name = "url", description = "Master URL for the env")]
    pub url: String,

    #[http_query(name = "id", description = "Container id")]
    pub id: String,
}

async fn handle_request(
    action: &GetProcessesAction,
    input_data: GetProcessesInputModel,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let fl_url = action
        .app
        .get_fl_url(input_data.env.as_str(), input_data.url.as_str())
        .await;

    let result = crate::http_client::get_processes(fl_url, input_data.id)
        .await
        .map_err(|err| HttpFailResult::as_fatal_error(err))?;

    HttpOutput::as_json(result)
        .with_compression(1024)
        .into_ok_result(false)
        .into()
}
