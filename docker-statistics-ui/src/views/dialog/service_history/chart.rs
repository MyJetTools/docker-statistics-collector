use std::rc::Rc;

use dioxus::prelude::*;

use super::{
    fmt_value, nearest_point, point_time_label, x_tick_label, x_tick_times, LoadedChart, VIEW_H,
    VIEW_W,
};

/// The history of one metric: the line through the averages of its points, the
/// band between their minimum and maximum, the limit when it is near enough to
/// matter — and, under the pointer, a readout of the point it is nearest to.
///
/// Three layers, so that the pointer moving redraws only the readout: the plot
/// and the strips that catch the pointer are given nothing that changes with it.
#[component]
pub fn HistoryChart(
    chart: Rc<LoadedChart>,
    hover: Option<usize>,
    dimmed: bool,
    on_hover: EventHandler<Option<usize>>,
) -> Element {
    let class = if dimmed { "hist-chart dimmed" } else { "hist-chart" };

    rsx! {
        div { class: "{class}",
            HistoryAxisY { chart: chart.clone() }
            div { class: "hist-plot",
                HistoryPlot { chart: chart.clone() }
                HistoryReadout { chart: chart.clone(), hover }
                HistoryHitArea { slots: chart.slots, on_hover }
            }
            HistoryAxisX { chart: chart.clone() }
        }
    }
}

#[component]
fn HistoryPlot(chart: Rc<LoadedChart>) -> Element {
    let metric = chart.metric.as_key();
    let view_box = format!("0 0 {} {}", VIEW_W, VIEW_H);

    let grid: Vec<f64> = chart
        .scale
        .ticks
        .iter()
        .map(|tick| VIEW_H - chart.scale.ratio(*tick) * VIEW_H)
        .collect();
    let limit_y = chart
        .scale
        .limit
        .map(|limit| VIEW_H - chart.scale.ratio(limit) * VIEW_H);

    rsx! {
        svg {
            class: "hist-svg",
            view_box: "{view_box}",
            preserve_aspect_ratio: "none",
            for y in grid {
                line { class: "hist-grid", x1: "0", x2: "{VIEW_W}", y1: "{y:.2}", y2: "{y:.2}" }
            }
            path { class: "hist-band {metric}", d: "{chart.paths.band}" }
            path { class: "hist-line {metric}", d: "{chart.paths.line}" }
            if let Some(y) = limit_y {
                line { class: "hist-limit", x1: "0", x2: "{VIEW_W}", y1: "{y:.2}", y2: "{y:.2}" }
            }
        }
        if let Some(limit) = chart.scale.limit {
            span {
                class: "hist-limit-label",
                style: "bottom: {chart.scale.ratio(limit) * 100.0:.2}%;",
                "limit {fmt_value(limit, &chart.scale.unit)}"
            }
        }
    }
}

/// The marks of the vertical axis, in the unit of the chart.
#[component]
fn HistoryAxisY(chart: Rc<LoadedChart>) -> Element {
    let unit = chart.scale.unit;

    rsx! {
        div { class: "hist-y",
            for tick in chart.scale.ticks.iter().copied() {
                span {
                    class: "hist-y-tick",
                    style: "bottom: {chart.scale.ratio(tick) * 100.0:.2}%;",
                    "{chart.scale.fmt_tick(tick)}"
                }
            }
            if !unit.suffix.is_empty() {
                span { class: "hist-y-unit", "{unit.suffix}" }
            }
        }
    }
}

/// The marks of the time axis — round moments of the viewer's own clock.
#[component]
fn HistoryAxisX(chart: Rc<LoadedChart>) -> Element {
    let span = chart.to - chart.from;
    let offset_seconds = crate::local_time::time_zone().offset_in_minutes() as i64 * 60;

    let ticks: Vec<(f64, String)> = x_tick_times(chart.from, chart.to, offset_seconds)
        .into_iter()
        .map(|time| {
            let stamp = crate::local_time::local_from_unix_seconds(time);
            (
                (time - chart.from) as f64 / span.max(1) as f64 * 100.0,
                x_tick_label(stamp.as_str(), span),
            )
        })
        .collect();

    rsx! {
        div { class: "hist-x",
            for (left , label) in ticks {
                span { class: "hist-x-tick", style: "left: {left:.2}%;", "{label}" }
            }
        }
    }
}

/// What the pointer is over: a hairline at the moment, a dot on the line, and
/// the figures of that point. The pointer aims at a moment, never at the line.
#[component]
fn HistoryReadout(chart: Rc<LoadedChart>, hover: Option<usize>) -> Element {
    let Some(index) = hover.and_then(|slot| nearest_point(&chart.data, chart.from, slot)) else {
        return rsx! {};
    };

    let unit = chart.scale.unit;
    let metric = chart.metric.as_key();
    let data = &chart.data;

    let left = chart.x_of(index) * 100.0;
    let bottom = chart.scale.ratio(data.avg[index]) * 100.0;
    // The readout opens away from the nearer edge, so it is never cut by it
    let tip_class = if left > 60.0 { "hist-tip left" } else { "hist-tip" };

    let stamp = crate::local_time::local_from_unix_seconds(data.time[index] as i64);
    let when = point_time_label(stamp.as_str(), data.step);

    rsx! {
        div { class: "hist-cross", style: "left: {left:.2}%;" }
        div { class: "hist-dot {metric}", style: "left: {left:.2}%; bottom: {bottom:.2}%;" }
        div { class: "{tip_class}", style: "left: {left:.2}%;",
            div { class: "when", "{when}" }
            div { class: "row",
                span { class: "key line {metric}" }
                b { "{fmt_value(data.avg[index], &unit)}" }
                span { class: "what", "avg" }
            }
            div { class: "row",
                span { class: "key band {metric}" }
                b { "{fmt_value(data.max[index], &unit)}" }
                span { class: "what", "max" }
            }
            div { class: "row",
                span { class: "key band {metric}" }
                b { "{fmt_value(data.min[index], &unit)}" }
                span { class: "what", "min" }
            }
            if let Some(limit) = data.limit[index] {
                div { class: "row",
                    span { class: "key limit" }
                    b { "{fmt_value(limit, &unit)}" }
                    span { class: "what", "limit" }
                }
            }
        }
    }
}

/// One invisible strip per stretch of the chart. Entering a strip is how the
/// pointer says where it is — no measuring of the element, no arithmetic on
/// pixels, and it stays right however the dialog is resized.
#[component]
fn HistoryHitArea(slots: usize, on_hover: EventHandler<Option<usize>>) -> Element {
    rsx! {
        div {
            class: "hist-hit",
            onmouseleave: move |_| on_hover.call(None),
            for slot in 0..slots {
                div {
                    key: "{slot}",
                    class: "hist-slot",
                    onmouseenter: move |_| on_hover.call(Some(slot)),
                }
            }
        }
    }
}
