use crate::app::AppContext;

use my_http_server::{macros::MyHttpInput, HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use std::sync::Arc;

use super::route_logs::{route_logs, RouteLogsResult};

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
    match route_logs(&action.app, input_data.id.as_str(), input_data.lines_number).await {
        RouteLogsResult::Ok(content) => HttpOutput::Content {
            status_code: 200,
            headers: Default::default(),
            content,
        }
        .into_ok_result(false),
        RouteLogsResult::NotFound => Err(HttpFailResult::as_not_found(
            format!("container {} not found on this instance or any peer", input_data.id),
            false,
        )),
        RouteLogsResult::PeerError(err) => Err(HttpFailResult::as_fatal_error(err)),
    }
}

#[derive(MyHttpInput)]
pub struct GetLogsHttpInput {
    #[http_query(description:"Container id")]
    pub id: String,
    #[http_query(description:"number of lines to return (from the tail)")]
    pub lines_number: u32,
}
