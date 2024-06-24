use crate::app::AppContext;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use std::sync::Arc;

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/api/metrics/list",
    description: "Get List Services with Metrics",
    summary: "Get List Services with Metrics",
    controller: "Metrics",
    result:[
        {status_code: 200, description: "List of services withMetrics", model:"Vec<String>" },
    ]
)]
pub struct GetListOfServicesWithMetrics {
    app: Arc<AppContext>,
}

impl GetListOfServicesWithMetrics {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetListOfServicesWithMetrics,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let containers = action.app.metrics_cache.get_list_of_services().await;

    HttpOutput::as_json(containers).into_ok_result(false).into()
}
