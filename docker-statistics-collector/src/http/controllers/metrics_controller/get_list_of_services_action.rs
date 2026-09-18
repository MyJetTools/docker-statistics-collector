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
    // Same live scrape as `/metrics`, reporting only the services that actually
    // answered with Prometheus content — same meaning the cached list had.
    let services: Vec<String> = crate::metrics_scraper::scrape_all(&action.app)
        .await
        .into_iter()
        .map(|itm| itm.service_name)
        .collect();

    HttpOutput::as_json(services).into_ok_result(false).into()
}
