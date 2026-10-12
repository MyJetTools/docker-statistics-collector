use std::sync::Arc;

use my_http_server::{
    macros::{http_route, MyHttpInput},
    HttpContext, HttpFailResult, HttpOkResult,
};

use crate::app::AppCtx;
use crate::metrics_storage::VM_SERIES;

use super::{answer_chart, ChartQuery};

#[http_route(
    method: "GET",
    route: "/api/vm-metrics/chart",
    controller: "ServiceMetrics",
    description: "Returns one metric of a machine as a whole - what its containers came to, poll by poll - over an interval of up to 31 days, as a chart of no more than `points` points. The same answer as the chart of a service. The cpu is measured against the cores of the host and the memory against its RAM; open files and throughput have no limit.",
    summary: "Chart of a metric of a machine",
    input_data: GetVmMetricsChartInputModel,
    result:[
        {status_code: 200, description: "ServiceMetricsChartResponse JSON"},
    ]
)]
pub struct GetVmMetricsChartAction {
    app: Arc<AppCtx>,
}

impl GetVmMetricsChartAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

#[derive(MyHttpInput)]
pub struct GetVmMetricsChartInputModel {
    #[http_query(name = "env", description = "Environment name")]
    pub env: String,

    #[http_query(name = "vm", description = "VM / instance name (ENV_INFO)")]
    pub vm: String,

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
    action: &GetVmMetricsChartAction,
    input_data: GetVmMetricsChartInputModel,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    answer_chart(
        &action.app,
        ctx,
        ChartQuery {
            env: input_data.env,
            vm: input_data.vm,
            series: VM_SERIES.to_string(),
            metric: input_data.metric,
            from: input_data.from,
            to: input_data.to,
            points: input_data.points,
        },
    )
    .await
}
