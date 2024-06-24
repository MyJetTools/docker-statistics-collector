use crate::app::AppContext;

use my_http_server::{
    macros::MyHttpInput, HttpContext, HttpFailResult, HttpOkResult, HttpOutput, WebContentType,
};

use std::sync::Arc;

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/api/metrics",
    description: "Get Prometheus metrics",
    summary: "Get Prometheus metrics",
    controller: "Metrics",
    input_data: GetMetricsContentHttpModel,
    result:[
        {status_code: 200, description: "Prometheus metrics", model:"String" },
    ]
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
    input_data: GetMetricsContentHttpModel,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    match action.app.metrics_cache.get_content(&input_data.id).await {
        Some(content) => HttpOutput::Content {
            content: content,
            content_type: WebContentType::Text.into(),
            headers: None,
        }
        .into_ok_result(false)
        .into(),

        None => Err(HttpFailResult::as_not_found(
            format!("No metrics found for service {}", input_data.id),
            false,
        )),
    }
}

#[derive(MyHttpInput)]
pub struct GetMetricsContentHttpModel {
    #[http_query(description: "Container id")]
    pub id: String,
}
