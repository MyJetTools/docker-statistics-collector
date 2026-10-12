use serde::Deserialize;

/// One metric of one service as a chart, as the hub's `/api/service-metrics/chart`
/// answers it: columns of one length, in time order. A point stands for `step`
/// seconds starting at its `time`; a stretch nothing was measured in has no point.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct ServiceMetricsChart {
    pub step: i64,
    pub time: Vec<u32>,
    pub avg: Vec<f64>,
    pub min: Vec<f64>,
    pub max: Vec<f64>,
    /// `None` where the value has no limit.
    pub limit: Vec<Option<f64>>,
}
