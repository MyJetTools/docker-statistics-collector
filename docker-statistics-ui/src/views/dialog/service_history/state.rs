use std::rc::Rc;

use crate::models::ServiceMetricsChart;
use crate::states::{HistoryMetric, HistoryPeriod};

use super::{build_paths, x_ratio, ChartPaths, YScale};

/// A chart as it arrived, together with everything drawing it needs — worked out
/// once, when the answer lands, so that a pointer moving over the chart costs
/// nothing but the readout.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedChart {
    pub metric: HistoryMetric,
    /// The interval the chart covers, unix seconds.
    pub from: i64,
    pub to: i64,
    pub data: ServiceMetricsChart,
    pub scale: YScale,
    pub paths: ChartPaths,
    /// The stretches the interval is cut into — the positions the pointer can aim at.
    pub slots: usize,
    /// The limit of the latest point, drawn or not.
    pub last_limit: Option<f64>,
    pub last: Option<f64>,
    pub avg: Option<f64>,
    pub max: Option<f64>,
}

impl LoadedChart {
    pub fn new(metric: HistoryMetric, from: i64, to: i64, data: ServiceMetricsChart) -> Self {
        let max = data.max.iter().copied().fold(None, |max: Option<f64>, value| {
            Some(max.map_or(value, |max| max.max(value)))
        });
        let avg = if data.avg.is_empty() {
            None
        } else {
            Some(data.avg.iter().sum::<f64>() / data.avg.len() as f64)
        };
        let last_limit = data.limit.last().copied().flatten();

        let scale = YScale::new(metric, max.unwrap_or(0.0), last_limit);
        let paths = build_paths(&data, from, to, &scale);
        let step = data.step.max(1);

        Self {
            metric,
            from,
            to,
            scale,
            paths,
            slots: (((to - from).max(1) + step - 1) / step) as usize,
            last_limit,
            last: data.avg.last().copied(),
            avg,
            max,
            data,
        }
    }

    /// Where the point number `index` is across the plot, 0..1.
    pub fn x_of(&self, index: usize) -> f64 {
        x_ratio(self.data.time[index] as i64, self.data.step, self.from, self.to)
    }
}

/// The state of the history dialog. One signal: what is picked, what is on
/// screen, and where the pointer is.
pub struct ServiceHistoryState {
    pub metric: HistoryMetric,
    pub period: HistoryPeriod,
    /// What is drawn. While a load is on its way this is still the previous
    /// answer — the frame is kept and dimmed rather than blanked.
    pub chart: Option<Rc<LoadedChart>>,
    pub loading: bool,
    pub error: Option<String>,
    /// The stretch of the chart the pointer is over.
    pub hover: Option<usize>,
    /// Counts the loads, so an answer that was overtaken by a newer request is
    /// recognised and dropped.
    request_no: u32,
}

impl ServiceHistoryState {
    pub fn new(metric: HistoryMetric) -> Self {
        Self {
            metric,
            period: HistoryPeriod::Hour,
            chart: None,
            loading: false,
            error: None,
            hover: None,
            request_no: 0,
        }
    }

    pub fn select_metric(&mut self, metric: HistoryMetric) {
        self.metric = metric;
        self.hover = None;
    }

    pub fn select_period(&mut self, period: HistoryPeriod) {
        self.period = period;
        self.hover = None;
    }

    pub fn set_hover(&mut self, hover: Option<usize>) {
        self.hover = hover;
    }

    /// A load begins. `quiet` is the periodic refresh: nothing the user asked
    /// for, so nothing on screen may flinch for it.
    pub fn load_started(&mut self, quiet: bool) -> (u32, HistoryMetric, HistoryPeriod) {
        self.request_no = self.request_no.wrapping_add(1);
        if !quiet {
            self.loading = true;
        }

        (self.request_no, self.metric, self.period)
    }

    pub fn load_finished(
        &mut self,
        request_no: u32,
        metric: HistoryMetric,
        from: i64,
        to: i64,
        result: Result<ServiceMetricsChart, String>,
    ) {
        if request_no != self.request_no {
            return;
        }

        self.loading = false;

        match result {
            Ok(data) => {
                self.chart = Some(Rc::new(LoadedChart::new(metric, from, to, data)));
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(values: &[f64]) -> ServiceMetricsChart {
        ServiceMetricsChart {
            step: 10,
            time: (0..values.len()).map(|index| 1000 + index as u32 * 10).collect(),
            avg: values.to_vec(),
            min: values.to_vec(),
            max: values.iter().map(|value| value * 2.0).collect(),
            limit: values.iter().map(|_| Some(100.0)).collect(),
        }
    }

    #[test]
    fn an_answer_overtaken_by_a_newer_request_is_dropped() {
        let mut state = ServiceHistoryState::new(HistoryMetric::Cpu);

        let (first, ..) = state.load_started(false);
        state.select_metric(HistoryMetric::Mem);
        let (second, metric, _) = state.load_started(false);

        // The answer to the first request lands after the second was sent
        state.load_finished(first, HistoryMetric::Cpu, 1000, 1100, Ok(answer(&[1.0])));
        assert!(state.chart.is_none());
        assert!(state.loading);

        state.load_finished(second, metric, 1000, 1100, Ok(answer(&[2.0])));
        assert_eq!(state.chart.as_ref().unwrap().metric, HistoryMetric::Mem);
        assert!(!state.loading);
    }

    #[test]
    fn a_failed_load_keeps_what_was_on_screen() {
        let mut state = ServiceHistoryState::new(HistoryMetric::Cpu);

        let (request_no, metric, _) = state.load_started(false);
        state.load_finished(request_no, metric, 1000, 1100, Ok(answer(&[1.0, 3.0])));

        let (request_no, metric, _) = state.load_started(false);
        state.load_finished(request_no, metric, 1000, 1100, Err("the hub is away".to_string()));

        assert!(state.chart.is_some());
        assert_eq!(state.error.as_deref(), Some("the hub is away"));
    }

    #[test]
    fn the_periodic_refresh_does_not_dim_the_chart() {
        let mut state = ServiceHistoryState::new(HistoryMetric::Cpu);
        state.load_started(true);
        assert!(!state.loading);
    }

    #[test]
    fn the_figures_of_a_chart_are_taken_over_all_of_its_points() {
        let chart = LoadedChart::new(HistoryMetric::Cpu, 1000, 1100, answer(&[1.0, 2.0, 6.0]));

        assert_eq!(chart.last, Some(6.0));
        assert_eq!(chart.avg, Some(3.0));
        assert_eq!(chart.max, Some(12.0));
        assert_eq!(chart.last_limit, Some(100.0));
        assert_eq!(chart.slots, 10);
        // 100 is more than four times the highest value: named, not drawn
        assert_eq!(chart.scale.limit, None);

        let empty = LoadedChart::new(HistoryMetric::Cpu, 1000, 1100, ServiceMetricsChart::default());
        assert_eq!((empty.last, empty.avg, empty.max), (None, None, None));
    }
}
