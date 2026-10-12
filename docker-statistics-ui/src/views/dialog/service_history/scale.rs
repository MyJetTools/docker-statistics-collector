use crate::models::ServiceMetricsChart;
use crate::states::HistoryMetric;

/// The plot is drawn in a box of this size and stretched to whatever the dialog
/// gives it. Strokes are told not to scale, so the stretch costs them nothing.
pub const VIEW_W: f64 = 1000.0;
pub const VIEW_H: f64 = 100.0;

/// A limit is drawn only while it is within this many times the highest value.
/// Past that, fitting it in would flatten the line the chart is there to show —
/// a container using half a core of a 32-core host — so it is named in the
/// figures above the chart instead.
const LIMIT_IN_VIEW_RATIO: f64 = 4.0;

/// How the values of a metric are written: what a raw value is divided by, and
/// the unit that follows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueUnit {
    pub divisor: f64,
    pub suffix: &'static str,
}

const KIB: f64 = 1024.0;
const MIB: f64 = 1024.0 * 1024.0;
const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

/// The unit a chart whose values reach `top` reads best in. One unit for the
/// whole chart: an axis which changes its unit half way up cannot be read.
pub fn value_unit(metric: HistoryMetric, top: f64) -> ValueUnit {
    let (divisor, suffix) = match metric {
        // What the collector reports is cores in use, not a percentage
        HistoryMetric::Cpu => (1.0, "cores"),
        HistoryMetric::Mem if top >= GIB => (GIB, "GiB"),
        HistoryMetric::Mem if top >= MIB => (MIB, "MiB"),
        HistoryMetric::Mem => (KIB, "KiB"),
        HistoryMetric::Files => (1.0, ""),
        // The hub keeps throughput in MB/s; the suffixes are the ones the rest of
        // the page uses for it
        HistoryMetric::NetIn | HistoryMetric::NetOut if top >= 1.0 => (1.0, "Mb/s"),
        HistoryMetric::NetIn | HistoryMetric::NetOut if top >= 1.0 / KIB => (1.0 / KIB, "Kb/s"),
        HistoryMetric::NetIn | HistoryMetric::NetOut => (1.0 / MIB, "b/s"),
    };

    ValueUnit { divisor, suffix }
}

/// A raw value in the chart's unit, with as many decimals as its size leaves
/// meaningful: `0.42`, `12.5`, `340`.
pub fn fmt_value(value: f64, unit: &ValueUnit) -> String {
    let number = fmt_number(value / unit.divisor);

    if unit.suffix.is_empty() {
        number
    } else {
        format!("{} {}", number, unit.suffix)
    }
}

pub fn fmt_number(value: f64) -> String {
    let abs = value.abs();

    if abs >= 100.0 || value.fract() == 0.0 {
        format!("{:.0}", value)
    } else if abs >= 10.0 {
        format!("{:.1}", value)
    } else {
        format!("{:.2}", value)
    }
}

/// The vertical axis: where it tops out, the lines it is ruled with and the
/// limit, when it is near enough to be drawn. All in raw values.
#[derive(Debug, Clone, PartialEq)]
pub struct YScale {
    pub top: f64,
    pub ticks: Vec<f64>,
    pub unit: ValueUnit,
    pub limit: Option<f64>,
    /// The decimals every mark of the axis is written with — as many as its
    /// step needs, and the same for all of them: `0.5 · 1.0 · 1.5`, not `0.5 · 1 · 1.5`.
    pub tick_decimals: usize,
}

impl YScale {
    pub fn new(metric: HistoryMetric, data_max: f64, limit: Option<f64>) -> Self {
        let data_max = if data_max.is_finite() { data_max.max(0.0) } else { 0.0 };

        let limit = limit.filter(|limit| {
            limit.is_finite()
                && *limit > 0.0
                && data_max > 0.0
                && *limit <= data_max * LIMIT_IN_VIEW_RATIO
        });

        let highest = limit.map_or(data_max, |limit| limit.max(data_max));
        let unit = value_unit(metric, highest);

        // Nothing but zeroes still gets an axis to be drawn on
        let highest = if highest > 0.0 {
            highest / unit.divisor
        } else {
            1.0
        };

        let mut step = nice_step(highest / 4.0);
        if metric == HistoryMetric::Files {
            // There is no such thing as half an open file
            step = step.max(1.0).ceil();
        }

        // A hair of room above the highest value, so its line is not the frame
        let steps = (highest * 1.02 / step).ceil().max(1.0) as usize;

        Self {
            top: steps as f64 * step * unit.divisor,
            ticks: (0..=steps)
                .map(|index| index as f64 * step * unit.divisor)
                .collect(),
            unit,
            limit,
            tick_decimals: decimals_of(step),
        }
    }

    /// A mark of the axis, in the unit of the chart.
    pub fn fmt_tick(&self, tick: f64) -> String {
        format!("{:.*}", self.tick_decimals, tick / self.unit.divisor)
    }

    /// Where a value sits between the bottom (0) and the top (1) of the plot.
    pub fn ratio(&self, value: f64) -> f64 {
        if self.top <= 0.0 {
            return 0.0;
        }

        (value / self.top).clamp(0.0, 1.0)
    }
}

/// The round number an axis is ruled by: 1, 2, 2.5 or 5 times a power of ten,
/// the first of them that is not below `raw`.
pub fn nice_step(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }

    let magnitude = 10f64.powf(raw.log10().floor());
    let fraction = raw / magnitude;

    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 2.5 {
        2.5
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice * magnitude
}

/// The decimals it takes to write a step of the axis exactly.
fn decimals_of(step: f64) -> usize {
    (0..3)
        .find(|decimals| {
            let scaled = step * 10f64.powi(*decimals as i32);
            (scaled - scaled.round()).abs() < 1e-9
        })
        .unwrap_or(3)
}

/// Two points further apart than this are not joined: the service measured
/// nothing in between, and a line across would say it did.
pub fn gap_seconds(step: i64) -> i64 {
    (step * 2).max(15)
}

/// Where the middle of the stretch that starts at `time` sits across the plot,
/// 0..1.
pub fn x_ratio(time: i64, step: i64, from: i64, to: i64) -> f64 {
    let span = (to - from).max(1) as f64;

    ((time + step / 2 - from) as f64 / span).clamp(0.0, 1.0)
}

/// The SVG paths of a chart: the line through the averages, and the band between
/// the minimum and the maximum of every point. Both break where the data does.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChartPaths {
    pub line: String,
    pub band: String,
}

pub fn build_paths(chart: &ServiceMetricsChart, from: i64, to: i64, scale: &YScale) -> ChartPaths {
    let mut result = ChartPaths::default();

    let x = |index: usize| x_ratio(chart.time[index] as i64, chart.step, from, to) * VIEW_W;
    let y = |value: f64| VIEW_H - scale.ratio(value) * VIEW_H;

    let gap = gap_seconds(chart.step);
    let len = chart.time.len();
    let mut start = 0;

    while start < len {
        // A run of points with no gap between them
        let mut end = start + 1;
        while end < len && chart.time[end] as i64 - chart.time[end - 1] as i64 <= gap {
            end += 1;
        }

        for index in start..end {
            let command = if index == start { 'M' } else { 'L' };
            result
                .line
                .push_str(&format!("{}{:.2} {:.2} ", command, x(index), y(chart.avg[index])));
        }
        if end - start == 1 {
            // A point on its own has no line to be seen by: a zero-length one,
            // which the round cap of the stroke turns into a dot
            result
                .line
                .push_str(&format!("L{:.2} {:.2} ", x(start), y(chart.avg[start])));
        }

        // Along the maximums, then back along the minimums
        for index in start..end {
            let command = if index == start { 'M' } else { 'L' };
            result
                .band
                .push_str(&format!("{}{:.2} {:.2} ", command, x(index), y(chart.max[index])));
        }
        for index in (start..end).rev() {
            result
                .band
                .push_str(&format!("L{:.2} {:.2} ", x(index), y(chart.min[index])));
        }
        result.band.push_str("Z ");

        start = end;
    }

    result
}

/// The seconds between two marks of the time axis, by how long the chart is.
pub fn x_tick_interval(span: i64) -> i64 {
    const HOUR: i64 = 3600;

    if span <= HOUR {
        10 * 60
    } else if span <= 6 * HOUR {
        HOUR
    } else if span <= 24 * HOUR {
        4 * HOUR
    } else if span <= 3 * 24 * HOUR {
        12 * HOUR
    } else {
        24 * HOUR
    }
}

/// The moments the time axis is marked at: round ones on the viewer's own clock
/// — `offset_seconds` is how far it is ahead of UTC — not on the server's.
pub fn x_tick_times(from: i64, to: i64, offset_seconds: i64) -> Vec<i64> {
    let interval = x_tick_interval(to - from);

    let mut local = (from + offset_seconds).div_euclid(interval) * interval;
    if local < from + offset_seconds {
        local += interval;
    }

    let mut result = Vec::new();
    while local - offset_seconds < to {
        result.push(local - offset_seconds);
        local += interval;
    }

    result
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `Oct 12` out of a `2026-10-12 14:30:05` stamp.
pub fn short_date(stamp: &str) -> String {
    let month = stamp
        .get(5..7)
        .and_then(|month| month.parse::<usize>().ok())
        .and_then(|month| MONTHS.get(month.wrapping_sub(1)))
        .copied()
        .unwrap_or("");
    let day = stamp.get(8..10).unwrap_or("").trim_start_matches('0');

    format!("{} {}", month, day)
}

/// What a mark of the time axis reads: the time of day, or the date where a
/// chart of several days crosses midnight.
pub fn x_tick_label(stamp: &str, span: i64) -> String {
    let time = stamp.get(11..16).unwrap_or("");

    if span > 24 * 3600 && time == "00:00" {
        short_date(stamp)
    } else {
        time.to_string()
    }
}

/// When a point is, for the tooltip: `Oct 12 14:30`, with the seconds while a
/// point is shorter than a minute.
pub fn point_time_label(stamp: &str, step: i64) -> String {
    let time = if step < 60 {
        stamp.get(11..19)
    } else {
        stamp.get(11..16)
    };

    format!("{} {}", short_date(stamp), time.unwrap_or(""))
}

/// How long a point is, in words: `10 s`, `4 min`, `1 h`.
pub fn step_label(step: i64) -> String {
    if step < 60 {
        format!("{} s", step)
    } else if step < 3600 {
        format!("{} min", fmt_number(step as f64 / 60.0))
    } else {
        format!("{} h", fmt_number(step as f64 / 3600.0))
    }
}

/// The point nearest to the stretch number `slot` of the chart, if it has any.
/// The pointer aims at a moment; it does not have to find a line.
pub fn nearest_point(chart: &ServiceMetricsChart, from: i64, slot: usize) -> Option<usize> {
    if chart.time.is_empty() {
        return None;
    }

    let time = from + slot as i64 * chart.step;
    let after = chart.time.partition_point(|point| (*point as i64) < time);

    let candidates = [after.checked_sub(1), Some(after).filter(|index| *index < chart.time.len())];

    candidates
        .into_iter()
        .flatten()
        .min_by_key(|index| (chart.time[*index] as i64 - time).abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart(step: i64, points: &[(u32, f64, f64, f64)]) -> ServiceMetricsChart {
        ServiceMetricsChart {
            step,
            time: points.iter().map(|point| point.0).collect(),
            min: points.iter().map(|point| point.1).collect(),
            avg: points.iter().map(|point| point.2).collect(),
            max: points.iter().map(|point| point.3).collect(),
            limit: points.iter().map(|_| None).collect(),
        }
    }

    #[test]
    fn the_axis_is_ruled_by_round_numbers_and_ends_above_the_data() {
        let scale = YScale::new(HistoryMetric::Cpu, 0.87, None);
        assert_eq!(scale.ticks, vec![0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_eq!(scale.top, 1.0);
        // Every mark with the decimals the step needs, the whole ones included
        let marks: Vec<String> = scale.ticks.iter().map(|tick| scale.fmt_tick(*tick)).collect();
        assert_eq!(marks, vec!["0.00", "0.25", "0.50", "0.75", "1.00"]);

        let scale = YScale::new(HistoryMetric::Cpu, 1.14, None);
        let marks: Vec<String> = scale.ticks.iter().map(|tick| scale.fmt_tick(*tick)).collect();
        assert_eq!(marks, vec!["0.0", "0.5", "1.0", "1.5"]);

        // 3.4 GiB of memory: ruled in GiB, not in bytes
        let scale = YScale::new(HistoryMetric::Mem, 3.4 * GIB, None);
        assert_eq!(scale.unit.suffix, "GiB");
        assert_eq!(scale.ticks, vec![0.0, GIB, 2.0 * GIB, 3.0 * GIB, 4.0 * GIB]);
        assert_eq!(scale.fmt_tick(3.0 * GIB), "3");
    }

    #[test]
    fn open_files_are_counted_in_whole_ones() {
        let scale = YScale::new(HistoryMetric::Files, 3.0, None);
        assert_eq!(scale.ticks, vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        assert_eq!(fmt_value(3.0, &scale.unit), "3");
    }

    #[test]
    fn a_limit_is_drawn_only_while_it_does_not_flatten_the_line() {
        // 41 GiB of a 48 GiB limit: the limit is the top of the picture
        let scale = YScale::new(HistoryMetric::Mem, 41.0 * GIB, Some(48.0 * GIB));
        assert_eq!(scale.limit, Some(48.0 * GIB));
        assert!(scale.top >= 48.0 * GIB);

        // Half a core of sixteen: drawing the limit would leave the line on the floor
        let scale = YScale::new(HistoryMetric::Cpu, 0.5, Some(16.0));
        assert_eq!(scale.limit, None);
        assert!(scale.top <= 1.0);
    }

    #[test]
    fn a_chart_of_nothing_still_has_an_axis() {
        let scale = YScale::new(HistoryMetric::NetIn, 0.0, None);
        assert!(scale.top > 0.0);
        assert_eq!(scale.ratio(0.0), 0.0);
    }

    #[test]
    fn values_are_written_in_the_unit_of_the_chart() {
        let unit = value_unit(HistoryMetric::Mem, 900.0 * MIB);
        assert_eq!(fmt_value(340.0 * MIB, &unit), "340 MiB");
        assert_eq!(fmt_value(12.5 * MIB, &unit), "12.5 MiB");

        let unit = value_unit(HistoryMetric::NetOut, 0.4);
        assert_eq!(fmt_value(0.25, &unit), "256 Kb/s");

        let unit = value_unit(HistoryMetric::Cpu, 2.0);
        assert_eq!(fmt_value(0.42, &unit), "0.42 cores");
    }

    #[test]
    fn the_line_breaks_where_nothing_was_measured() {
        // Three points ten seconds apart, a hole of a minute, then one on its own
        let chart = chart(
            10,
            &[
                (1000, 1.0, 2.0, 3.0),
                (1010, 1.0, 2.0, 3.0),
                (1020, 1.0, 2.0, 3.0),
                (1080, 1.0, 2.0, 3.0),
            ],
        );
        let scale = YScale::new(HistoryMetric::Cpu, 3.0, None);
        let paths = build_paths(&chart, 1000, 1100, &scale);

        assert_eq!(paths.line.matches('M').count(), 2);
        assert_eq!(paths.band.matches('Z').count(), 2);
        // The lone point is a zero-length line, which the round cap makes a dot
        assert_eq!(paths.line.matches('L').count(), 3);
    }

    #[test]
    fn a_point_is_drawn_in_the_middle_of_its_stretch() {
        assert_eq!(x_ratio(1000, 10, 1000, 1100), 0.05);
        assert_eq!(x_ratio(1090, 10, 1000, 1100), 0.95);
    }

    #[test]
    fn the_time_axis_is_marked_on_the_viewers_clock() {
        // An hour from 12:03 UTC: every ten minutes
        let from = 1_791_763_200 + 12 * 3600 + 180;
        let ticks = x_tick_times(from, from + 3600, 0);
        assert_eq!(ticks.len(), 6);
        assert_eq!((ticks[0] - 1_791_763_200) % 86_400, 12 * 3600 + 600);

        // A week, seen from UTC+3: the marks are at the viewer's midnights
        let ticks = x_tick_times(from, from + 7 * 86_400, 3 * 3600);
        assert_eq!(ticks.len(), 7);
        assert!(ticks.iter().all(|tick| (tick + 3 * 3600) % 86_400 == 0));
    }

    #[test]
    fn a_chart_of_days_names_the_date_at_midnight() {
        assert_eq!(x_tick_label("2026-10-12 00:00:00", 3 * 86_400), "Oct 12");
        assert_eq!(x_tick_label("2026-10-12 12:00:00", 3 * 86_400), "12:00");
        assert_eq!(x_tick_label("2026-10-12 00:00:00", 3600), "00:00");
        assert_eq!(point_time_label("2026-10-05 14:30:10", 10), "Oct 5 14:30:10");
        assert_eq!(point_time_label("2026-10-05 14:30:00", 240), "Oct 5 14:30");
    }

    #[test]
    fn the_pointer_snaps_to_the_nearest_point() {
        let chart = chart(10, &[(1000, 0.0, 0.0, 0.0), (1010, 0.0, 0.0, 0.0), (1080, 0.0, 0.0, 0.0)]);

        assert_eq!(nearest_point(&chart, 1000, 0), Some(0));
        assert_eq!(nearest_point(&chart, 1000, 1), Some(1));
        // In the hole: whichever side is closer
        assert_eq!(nearest_point(&chart, 1000, 3), Some(1));
        assert_eq!(nearest_point(&chart, 1000, 6), Some(2));
        assert_eq!(nearest_point(&chart, 1000, 9), Some(2));

        assert_eq!(nearest_point(&ServiceMetricsChart::default(), 1000, 3), None);
    }

    #[test]
    fn the_length_of_a_point_is_said_in_words() {
        assert_eq!(step_label(10), "10 s");
        assert_eq!(step_label(240), "4 min");
        assert_eq!(step_label(720), "12 min");
        assert_eq!(step_label(1680), "28 min");
        assert_eq!(step_label(3600), "1 h");
    }
}
