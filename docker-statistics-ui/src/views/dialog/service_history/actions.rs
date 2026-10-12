use dioxus::prelude::*;
use rust_extensions::date_time::DateTimeAsMicroseconds;

use crate::states::{HistoryPeriod, ServiceHistoryModel};

use super::ServiceHistoryState;

/// The points a chart is asked for: about one for every three pixels of the
/// dialog. Finer than that the hub would be averaging for nobody.
pub const CHART_POINTS: usize = 360;

/// Load the chart for what is picked in the dialog right now. `quiet` is the
/// periodic refresh, which must not dim what is on screen.
pub async fn load_history(
    mut cs: Signal<ServiceHistoryState>,
    target: ServiceHistoryModel,
    quiet: bool,
) {
    let (request_no, metric, period) = cs.write().load_started(quiet);

    let now = DateTimeAsMicroseconds::now().unix_microseconds / 1_000_000;
    let (from, to) = interval(period, now);

    let result = crate::api::get_service_metrics_chart(
        target.env.as_str(),
        target.vm.as_str(),
        target.service.as_str(),
        metric.as_key(),
        from,
        to,
        CHART_POINTS,
    )
    .await
    .map_err(|err| err.to_string());

    cs.write()
        .load_finished(request_no, metric, from, to, result);
}

/// The interval a period covers at `now`, ending on a whole point. With the end
/// on the grid of points the same moments fall into the same points from one
/// refresh to the next, so a chart being refreshed moves on — it does not shimmer.
pub fn interval(period: HistoryPeriod, now: i64) -> (i64, i64) {
    let span = period.seconds();
    let points = CHART_POINTS as i64;
    let step = (span + points - 1) / points;

    let to = (now.div_euclid(step) + 1) * step;

    (to - span, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_interval_ends_on_a_whole_point_just_after_now() {
        // An hour is 360 points of 10 seconds
        let (from, to) = interval(HistoryPeriod::Hour, 1_000_003);
        assert_eq!(to, 1_000_010);
        assert_eq!(to - from, 3600);

        // Three seconds later the grid has not moved
        assert_eq!(interval(HistoryPeriod::Hour, 1_000_006), (from, to));

        // A week is points of 28 minutes
        let (from, to) = interval(HistoryPeriod::Week, 1_000_003);
        assert_eq!(to % 1680, 0);
        assert!(to > 1_000_003 && to - 1_000_003 <= 1680);
        assert_eq!(to - from, 7 * 24 * 3600);
    }
}
