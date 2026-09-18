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
    // Scraped on demand — nothing is kept between calls.
    let scraped = crate::metrics_scraper::scrape_all(&action.app).await;

    let total: usize = scraped.iter().map(|itm| itm.content.len()).sum();
    let mut content = Vec::with_capacity(total);
    for itm in scraped {
        content.extend_from_slice(itm.content.as_slice());
    }

    HttpOutput::Content {
        status_code: 200,
        content,
        headers: WebContentType::Text.into(),
    }
    .into_ok_result(false)
    .into()
}
