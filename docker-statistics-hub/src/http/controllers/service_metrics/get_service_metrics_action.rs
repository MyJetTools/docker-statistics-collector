use std::sync::Arc;

use my_http_server::{
    macros::{http_route, MyHttpInput},
    HttpContext, HttpFailResult, HttpOkResult, HttpOutput,
};
use rust_extensions::date_time::DateTimeAsMicroseconds;
use serde::Serialize;

use crate::app::AppCtx;
use crate::metrics_storage::{Metric, Record};

/// The longest interval one request reads. The answer is built in memory, and a service
/// can have a measurement a second: two days are 172 800 of them at most - enough for a
/// day in any time zone, the 25 hours of a day the clocks go back on included.
const MAX_INTERVAL_SECONDS: i64 = 2 * 24 * 60 * 60;

#[http_route(
    method: "GET",
    route: "/api/service-metrics",
    controller: "ServiceMetrics",
    description: "Returns the measurements of one metric of one service for an interval of up to 48 hours, as they were taken: the second of each, its value and its limit. Read from the day files of the metrics storage; an interval nothing was measured in is an empty answer.",
    summary: "History of a metric of a service",
    input_data: GetServiceMetricsInputModel,
    result:[
        {status_code: 200, description: "ServiceMetricsResponse JSON"},
    ]
)]
pub struct GetServiceMetricsAction {
    app: Arc<AppCtx>,
}

impl GetServiceMetricsAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

#[derive(MyHttpInput)]
pub struct GetServiceMetricsInputModel {
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
}

/// The measurements as three columns of one length, in time order.
#[derive(Serialize)]
pub struct ServiceMetricsResponse {
    /// Unix seconds each measurement was taken at.
    pub time: Vec<u32>,
    pub value: Vec<f64>,
    /// `null` where the value has no limit.
    pub limit: Vec<Option<f64>>,
}

impl ServiceMetricsResponse {
    fn new(records: Vec<Record>) -> Self {
        let mut result = Self {
            time: Vec::with_capacity(records.len()),
            value: Vec::with_capacity(records.len()),
            limit: Vec::with_capacity(records.len()),
        };

        for record in records {
            result.time.push(record.time);
            result.value.push(record.value);
            // "No limit" is NaN in the files, and NaN is not a number JSON has
            result
                .limit
                .push(Some(record.limit).filter(|limit| limit.is_finite()));
        }

        result
    }
}

async fn handle_request(
    action: &GetServiceMetricsAction,
    input_data: GetServiceMetricsInputModel,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let user_id = crate::auth::user_from_http(ctx);

    let settings = action.app.settings_reader.get_settings().await;
    if !settings.is_env_allowed_for_user(&user_id, &input_data.env) {
        return Err(HttpFailResult::as_forbidden(Some(format!(
            "env '{}' is not accessible for user '{}'",
            input_data.env, user_id
        ))));
    }
    drop(settings);

    let Some(metric) = Metric::parse(input_data.metric.as_str()) else {
        return Err(HttpFailResult::as_validation_error(format!(
            "metric '{}' is not one of: {}",
            input_data.metric,
            Metric::ALL.map(|metric| metric.as_str()).join(", ")
        )));
    };

    if input_data.service.is_empty() {
        return Err(HttpFailResult::as_validation_error("service is required"));
    }

    let interval = input_data.to.saturating_sub(input_data.from);

    if interval <= 0 {
        return Err(HttpFailResult::as_validation_error(
            "to must be later than from",
        ));
    }

    if interval > MAX_INTERVAL_SECONDS {
        return Err(HttpFailResult::as_validation_error(format!(
            "the interval is {} seconds, and at most {} are read at once",
            interval, MAX_INTERVAL_SECONDS
        )));
    }

    let storage = action.app.metrics_storage.clone();

    let records = tokio::task::spawn_blocking(move || {
        let now = DateTimeAsMicroseconds::now().unix_microseconds / 1_000_000;

        storage.read(
            input_data.env.as_str(),
            input_data.vm.as_str(),
            input_data.service.as_str(),
            metric,
            input_data.from,
            input_data.to,
            now,
        )
    })
    .await
    .map_err(|err| HttpFailResult::as_fatal_error(format!("the read did not complete: {:?}", err)))?
    .map_err(HttpFailResult::as_fatal_error)?;

    HttpOutput::as_json(ServiceMetricsResponse::new(records))
        .with_compression(1024)
        .into_ok_result(false)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_limit_which_is_not_there_is_null_in_the_answer() {
        let response = ServiceMetricsResponse::new(vec![
            Record {
                time: 100,
                limit: 8.0,
                value: 0.5,
            },
            Record {
                time: 103,
                limit: f64::NAN,
                value: 1.5,
            },
        ]);

        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"time":[100,103],"value":[0.5,1.5],"limit":[8.0,null]}"#
        );
    }
}
