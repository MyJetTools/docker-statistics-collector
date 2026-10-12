use std::sync::Arc;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult, HttpOutput};
use rust_extensions::date_time::DateTimeAsMicroseconds;
use serde::Serialize;

use crate::app::AppCtx;
use crate::metrics_storage::{Chart, Metric};

/// The longest interval one chart covers. The records are read a day at a time and
/// folded into the points as they are read, so what bounds it is not memory but how
/// long a request may take: a month is thirty day files.
const MAX_INTERVAL_SECONDS: i64 = 31 * 24 * 60 * 60;

/// The points of a chart when the request does not say how many - about one for every
/// three pixels of a wide dialog.
const DEFAULT_POINTS: u32 = 360;

/// The most points a chart is given: more than a screen has pixels for is not drawn
const MAX_POINTS: u32 = 2000;

/// What a chart is asked of: a series - a service, or the machine as a whole - of a VM
/// of an env, one metric of it, over an interval.
pub struct ChartQuery {
    pub env: String,
    pub vm: String,
    /// What the series is kept under in the day files
    pub series: String,
    pub metric: String,
    pub from: i64,
    pub to: i64,
    pub points: Option<u32>,
}

/// The chart as columns of one length, in time order.
#[derive(Serialize)]
pub struct ServiceMetricsChartResponse {
    /// Seconds each point stands for.
    pub step: i64,
    /// Unix second each point starts at.
    pub time: Vec<u32>,
    pub avg: Vec<f64>,
    pub min: Vec<f64>,
    pub max: Vec<f64>,
    /// `null` where the value has no limit.
    pub limit: Vec<Option<f64>>,
}

impl ServiceMetricsChartResponse {
    fn new(chart: Chart) -> Self {
        let len = chart.points.len();

        let mut result = Self {
            step: chart.step,
            time: Vec::with_capacity(len),
            avg: Vec::with_capacity(len),
            min: Vec::with_capacity(len),
            max: Vec::with_capacity(len),
            limit: Vec::with_capacity(len),
        };

        for point in chart.points {
            result.time.push(point.time);
            result.avg.push(point.avg);
            result.min.push(point.min);
            result.max.push(point.max);
            // "No limit" is NaN in the files, and NaN is not a number JSON has
            result
                .limit
                .push(Some(point.limit).filter(|limit| limit.is_finite()));
        }

        result
    }
}

/// Checks the request and answers it with the chart. The same for a service and for a
/// machine: they are series of one kind, and differ in what they are kept under.
pub async fn answer_chart(
    app: &Arc<AppCtx>,
    ctx: &HttpContext,
    query: ChartQuery,
) -> Result<HttpOkResult, HttpFailResult> {
    let user_id = crate::auth::user_from_http(ctx);

    let settings = app.settings_reader.get_settings().await;
    if !settings.is_env_allowed_for_user(&user_id, &query.env) {
        return Err(HttpFailResult::as_forbidden(Some(format!(
            "env '{}' is not accessible for user '{}'",
            query.env, user_id
        ))));
    }
    drop(settings);

    let Some(metric) = Metric::parse(query.metric.as_str()) else {
        return Err(HttpFailResult::as_validation_error(format!(
            "metric '{}' is not one of: {}",
            query.metric,
            Metric::ALL.map(|metric| metric.as_str()).join(", ")
        )));
    };

    if query.vm.is_empty() {
        return Err(HttpFailResult::as_validation_error("vm is required"));
    }

    let interval = query.to.saturating_sub(query.from);

    if interval <= 0 {
        return Err(HttpFailResult::as_validation_error(
            "to must be later than from",
        ));
    }

    if interval > MAX_INTERVAL_SECONDS {
        return Err(HttpFailResult::as_validation_error(format!(
            "the interval is {} seconds, and a chart covers at most {}",
            interval, MAX_INTERVAL_SECONDS
        )));
    }

    let points = query.points.unwrap_or(DEFAULT_POINTS).clamp(1, MAX_POINTS) as usize;

    let storage = app.metrics_storage.clone();

    let chart = tokio::task::spawn_blocking(move || {
        let now = DateTimeAsMicroseconds::now().unix_microseconds / 1_000_000;

        storage.read_chart(
            query.env.as_str(),
            query.vm.as_str(),
            query.series.as_str(),
            metric,
            query.from,
            query.to,
            points,
            now,
        )
    })
    .await
    .map_err(|err| HttpFailResult::as_fatal_error(format!("the read did not complete: {:?}", err)))?
    .map_err(HttpFailResult::as_fatal_error)?;

    HttpOutput::as_json(ServiceMetricsChartResponse::new(chart))
        .with_compression(1024)
        .into_ok_result(false)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics_storage::ChartPoint;

    #[test]
    fn the_points_become_columns_and_a_limit_which_is_not_there_is_null() {
        let response = ServiceMetricsChartResponse::new(Chart {
            step: 10,
            points: vec![
                ChartPoint {
                    time: 100,
                    avg: 1.5,
                    min: 1.0,
                    max: 2.0,
                    limit: 8.0,
                },
                ChartPoint {
                    time: 130,
                    avg: 3.0,
                    min: 3.0,
                    max: 3.0,
                    limit: f64::NAN,
                },
            ],
        });

        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"step":10,"time":[100,130],"avg":[1.5,3.0],"min":[1.0,3.0],"max":[2.0,3.0],"limit":[8.0,null]}"#
        );
    }
}
