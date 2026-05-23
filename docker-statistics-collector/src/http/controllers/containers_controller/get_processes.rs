use crate::app::AppContext;
use crate::peers_client::{fanout_processes, RouteProcessesResult};

use my_http_server::{macros::MyHttpInput, HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use std::sync::Arc;

use super::contracts::ContainerProcessesHttpResponse;

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/api/containers/processes",
    description: "List a container's processes with per-process open file descriptors and nofile limit",
    summary: "Get container processes file descriptors",
    controller: "Containers",
    input_data: "GetProcessesHttpInput",
    result:[
        {status_code: 200, description: "Processes of container", model:"ContainerProcessesHttpResponse" },
    ]
)]
pub struct GetProcessesAction {
    app: Arc<AppContext>,
}

impl GetProcessesAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetProcessesAction,
    input_data: GetProcessesHttpInput,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    match fanout_processes(&action.app, input_data.id.as_str()).await {
        RouteProcessesResult::Ok(processes) => HttpOutput::as_json(ContainerProcessesHttpResponse {
            container_id: input_data.id,
            processes,
        })
        .into_ok_result(false)
        .into(),
        RouteProcessesResult::NotFound => Err(HttpFailResult::as_not_found(
            format!(
                "container {} not found on this instance or any peer",
                input_data.id
            ),
            false,
        )),
        RouteProcessesResult::PeerError(err) => Err(HttpFailResult::as_fatal_error(err)),
    }
}

#[derive(MyHttpInput)]
pub struct GetProcessesHttpInput {
    #[http_query(description:"Container id")]
    pub id: String,
}
