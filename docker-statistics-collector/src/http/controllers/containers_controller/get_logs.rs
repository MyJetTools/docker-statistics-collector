use crate::app::AppContext;

use my_http_server::{macros::MyHttpInput, HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use std::sync::Arc;

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/api/containers/logs",
    description: "Get containers logs",
    summary: "Get containers logs",
    controller: "Containers",
    input_data: "GetLogsHttpInput",
    result:[
        {status_code: 200, description: "Logs of container", model:"String" },
    ]
)]
pub struct GetLogsAction {
    app: Arc<AppContext>,
}

impl GetLogsAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetLogsAction,
    input_data: GetLogsHttpInput,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let url = action.app.settings_model.url.to_string();

    let mut result = Vec::new();

    for _ in 0..10 {
        let payload = docker_sdk::sdk::get_container_logs(
            url.as_str(),
            input_data.id.as_str(),
            input_data.lines_number,
        )
        .await;

        if payload.len() > result.len() {
            result = payload;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    HttpOutput::Content {
        headers: None,
        content_type: None,
        content: result,
    }
    .into_ok_result(false)
}

#[derive(MyHttpInput)]
pub struct GetLogsHttpInput {
    #[http_query(description:"Container id")]
    pub id: String,
    #[http_query(description:"number of lines to return (from the tail)")]
    pub lines_number: u32,
}
