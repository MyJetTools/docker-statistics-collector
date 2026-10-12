use std::sync::Arc;

use my_http_server::{
    macros::{http_route, MyHttpInput},
    HttpContext, HttpFailResult, HttpOkResult,
};

use crate::app::AppCtx;

use super::{answer_chart, ChartQuery};

#[http_route(
    method: "GET",
    route: "/api/service-metrics/chart",
    controller: "ServiceMetrics",
    description: "Returns one metric of one service over an interval of up to 31 days as a chart: the interval is cut into no more than `points` stretches of equal length, and each stretch something was measured in is a point - its average, minimum and maximum, and the limit of its latest measurement. A stretch nothing was measured in has no point.",
    summary: "Chart of a metric of a service",
    input_data: GetServiceMetricsChartInputModel,
    result:[
        {status_code: 200, description: "ServiceMetricsChartResponse JSON"},
    ]
)]
pub struct GetServiceMetricsChartAction {
    app: Arc<AppCtx>,
}

impl GetServiceMetricsChartAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

#[derive(MyHttpInput)]
pub struct GetServiceMetricsChartInputModel {
    #[http_query(name = "env", description = "Environment name")]
    pub env: String,

    #[http_query(
        name = "vm",
        description = "VM / instance name (ENV_INFO) the service runs on"
    )]
    pub vm: String,

    #[http_query(
        name = "service",
        description = "Name of the container, as the UI shows it"
    )]
    pub service: String,

    #[http_query(
        name = "metric",
        description = "One of: cpu, mem, files, net-in, net-out"
    )]
    pub metric: String,

    #[http_query(
        name = "from",
        description = "Start of the interval, unix seconds, inclusive"
    )]
    pub from: i64,

    #[http_query(
        name = "to",
        description = "End of the interval, unix seconds, exclusive"
    )]
    pub to: i64,

    #[http_query(
        name = "points",
        description = "The most points the chart has room for; 360 when absent, 2000 at most"
    )]
    pub points: Option<u32>,
}

async fn handle_request(
    action: &GetServiceMetricsChartAction,
    input_data: GetServiceMetricsChartInputModel,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    if input_data.service.is_empty() {
        return Err(HttpFailResult::as_validation_error("service is required"));
    }

    answer_chart(
        &action.app,
        ctx,
        ChartQuery {
            env: input_data.env,
            vm: input_data.vm,
            series: input_data.service,
            metric: input_data.metric,
            from: input_data.from,
            to: input_data.to,
            points: input_data.points,
        },
    )
    .await
}
