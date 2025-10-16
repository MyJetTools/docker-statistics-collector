use crate::app::AppContext;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult, HttpOutput, WebContentType};

use std::sync::Arc;

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/metrics",
)]
pub struct GetMetricsAction {
    app: Arc<AppContext>,
}

impl GetMetricsAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetMetricsAction,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let content = action.app.metrics_cache.get_aggregated_metrics().await;

    HttpOutput::Content {
        status_code: 200,
        content: content,
        content_type: WebContentType::Text.into(),
        headers: None,
        set_cookies: None,
    }
    .into_ok_result(false)
    .into()
}
