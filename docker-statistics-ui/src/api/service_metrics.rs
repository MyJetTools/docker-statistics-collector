use flurl::FlUrl;

use crate::models::{RequestError, ServiceMetricsChart};

use super::handle_http_response;

/// The history of one metric of one service over `from .. to` (unix seconds), cut
/// by the hub into no more than `points` points — it is the hub that thins a week
/// of measurements down to what a chart has room for, not the page.
pub async fn get_service_metrics_chart(
    env: &str,
    vm: &str,
    service: &str,
    metric: &str,
    from: i64,
    to: i64,
    points: usize,
) -> Result<ServiceMetricsChart, RequestError> {
    let response = FlUrl::new("/api/service-metrics/chart")
        .append_query_param("env", Some(env))
        .append_query_param("vm", Some(vm))
        .append_query_param("service", Some(service))
        .append_query_param("metric", Some(metric))
        .append_query_param("from", Some(from.to_string()))
        .append_query_param("to", Some(to.to_string()))
        .append_query_param("points", Some(points.to_string()))
        .get()
        .await;

    handle_http_response(response).await
}

/// The same for a machine as a whole: what its containers came to, poll by poll.
pub async fn get_vm_metrics_chart(
    env: &str,
    vm: &str,
    metric: &str,
    from: i64,
    to: i64,
    points: usize,
) -> Result<ServiceMetricsChart, RequestError> {
    let response = FlUrl::new("/api/vm-metrics/chart")
        .append_query_param("env", Some(env))
        .append_query_param("vm", Some(vm))
        .append_query_param("metric", Some(metric))
        .append_query_param("from", Some(from.to_string()))
        .append_query_param("to", Some(to.to_string()))
        .append_query_param("points", Some(points.to_string()))
        .get()
        .await;

    handle_http_response(response).await
}
