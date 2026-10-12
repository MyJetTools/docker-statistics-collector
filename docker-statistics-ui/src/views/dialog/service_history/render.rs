use std::time::Duration;

use dioxus::prelude::*;

use crate::states::{HistoryMetric, HistoryPeriod, ServiceHistoryModel};
use crate::views::dialog::{
    fmt_value, load_history, step_label, HistoryChart, ServiceHistoryState,
};

/// How often an open dialog asks again. The hub polls every 3 seconds; on a
/// chart whose finest point is ten, half a minute is as live as it can look.
const REFRESH_EVERY: Duration = Duration::from_secs(30);

/// The history of one service — or of a machine as a whole — as far back as the
/// hub keeps it: one metric at a time, over the period picked. Opened from the
/// name of a chart on the container panel, or from the card or the boards of a
/// machine, on the metric that was pressed.
#[component]
pub fn service_history(model: ServiceHistoryModel) -> Element {
    let mut cs = use_signal(|| ServiceHistoryState::new(model.metric));

    // The first load, then a quiet one every half a minute for as long as the
    // dialog is open — the future goes with the component.
    let target = model.clone();
    use_future(move || {
        let target = target.clone();
        async move {
            load_history(cs, target.clone(), false).await;
            loop {
                dioxus_utils::js::sleep(REFRESH_EVERY).await;
                load_history(cs, target.clone(), true).await;
            }
        }
    });

    let cs_ra = cs.read();
    let metric = cs_ra.metric;
    let period = cs_ra.period;
    let chart = cs_ra.chart.clone();
    let loading = cs_ra.loading;

    let caption = chart.as_ref().map(|chart| {
        format!(
            "one point is {} · times are {}",
            step_label(chart.data.step),
            crate::local_time::zone_label()
        )
    });

    rsx! {
        div { class: "hist",
            // Everything that scopes the chart, in one row above it
            div { class: "hist-controls",
                div { class: "filters",
                    for item in HistoryMetric::ALL {
                        button {
                            class: if item == metric { "chip active" } else { "chip" },
                            onclick: {
                                let target = model.clone();
                                move |_| {
                                    cs.write().select_metric(item);
                                    spawn(load_history(cs, target.clone(), false));
                                }
                            },
                            "{item.label()}"
                        }
                    }
                }
                div { class: "filters",
                    for item in HistoryPeriod::ALL {
                        button {
                            class: if item == period { "chip active" } else { "chip" },
                            onclick: {
                                let target = model.clone();
                                move |_| {
                                    cs.write().select_period(item);
                                    spawn(load_history(cs, target.clone(), false));
                                }
                            },
                            "{item.label()}"
                        }
                    }
                }
            }

            if let Some(error) = cs_ra.error.as_ref() {
                div { class: "ds-error", "{error}" }
            }

            match chart {
                Some(chart) => rsx! {
                    HistoryFigures { chart: chart.clone() }
                    if chart.data.time.is_empty() {
                        div { class: "hist-empty", "nothing was measured in this period" }
                    } else {
                        HistoryChart {
                            chart: chart.clone(),
                            hover: cs_ra.hover,
                            dimmed: loading,
                            on_hover: move |hover| {
                                // Entering the next strip is not news when it is the same one
                                if cs.peek().hover != hover {
                                    cs.write().set_hover(hover);
                                }
                            },
                        }
                    }
                },
                None if cs_ra.error.is_none() => rsx! {
                    div { class: "ds-loading", "Loading history…" }
                },
                None => rsx! {},
            }

            if let Some(caption) = caption {
                div { class: "hist-caption", "{caption}" }
            }
        }
    }
}

/// The figures of the period and what the marks of the chart mean. They say
/// without the pointer what the readout says with it.
#[component]
fn HistoryFigures(chart: std::rc::Rc<crate::views::dialog::LoadedChart>) -> Element {
    let unit = chart.scale.unit;
    let metric = chart.metric.as_key();
    let figure = |value: Option<f64>| {
        value
            .map(|value| fmt_value(value, &unit))
            .unwrap_or_else(|| "—".to_string())
    };

    rsx! {
        div { class: "hist-figures",
            div { class: "fig",
                span { class: "k", "last" }
                span { class: "v", "{figure(chart.last)}" }
            }
            div { class: "fig",
                span { class: "k", "average" }
                span { class: "v", "{figure(chart.avg)}" }
            }
            div { class: "fig",
                span { class: "k", "peak" }
                span { class: "v", "{figure(chart.max)}" }
            }
            if let Some(limit) = chart.last_limit {
                div { class: "fig",
                    span { class: "k", "limit" }
                    span { class: "v", "{fmt_value(limit, &unit)}" }
                }
            }
            div { class: "hist-legend",
                span { class: "item",
                    span { class: "key line {metric}" }
                    "average"
                }
                span { class: "item",
                    span { class: "key band {metric}" }
                    "min – max"
                }
                if chart.scale.limit.is_some() {
                    span { class: "item",
                        span { class: "key limit" }
                        "limit"
                    }
                }
            }
        }
    }
}
