use dioxus::prelude::*;

use crate::models::MetricsByVm;
use crate::router::AppRoute;
use crate::states::{MainState, TopMetric};
use crate::views::dockerscope::detail::{build_top_consumers, TopConsumerRow};
use crate::views::dockerscope::helpers::fmt_mem_pair;

/// Fills the detail column while a VM is selected but no container is: the
/// heaviest containers of that VM ranked by CPU on the left and by memory on
/// the right, so the two can be read against each other at a glance.
#[component]
pub fn TopConsumersPanel() -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let cs_ra = main_state.read();

    // In `/all` view each row carries its own VM; scoped to one VM the name is
    // implicit and lives in the URL prefix instead.
    let single_vm_name = if cs_ra.is_all_vms_selected() {
        None
    } else {
        cs_ra.get_selected_vm_name()
    };

    let containers = cs_ra.get_containers();
    let top_n = cs_ra.get_top_n();
    let top_n_raw = cs_ra.get_top_n_raw().to_string();

    rsx! {
        div { class: "tc-wrap",
            div { class: "tc-head",
                div { class: "ds-input-group tc-topn",
                    span { class: "label", "top" }
                    input {
                        r#type: "number",
                        min: "0",
                        value: "{top_n_raw}",
                        title: "rows per board — 0 shows every container",
                        // `oninput`, not `onchange`: nothing is fetched, so the
                        // boards can follow each keystroke instead of waiting
                        // for the field to lose focus.
                        oninput: move |evt| {
                            consume_context::<Signal<MainState>>().write().set_top_n(evt.value());
                        },
                    }
                }
                span { class: "tc-hint", "0 = all" }
            }

            div { class: "tc-boards",
                {render_board(TopMetric::Cpu, &containers, top_n, single_vm_name.clone())}
                {render_board(TopMetric::Mem, &containers, top_n, single_vm_name.clone())}
            }
        }
    }
}

/// One board. A plain function rather than a `#[component]` so the ranked set
/// can be passed by reference — it is rebuilt every poll and nothing is gained
/// by making it a prop that has to be cloned and compared.
fn render_board(
    metric: TopMetric,
    containers: &[&MetricsByVm],
    top_n: usize,
    single_vm_name: Option<String>,
) -> Element {
    let board = build_top_consumers(containers, metric, top_n);

    let total = match metric {
        TopMetric::Cpu => format!("{:.2}% total", board.total_cpu),
        TopMetric::Mem => {
            let (value, unit) = fmt_mem_pair(board.total_mem);
            format!("{} {} total", value, unit)
        }
    };
    let summary = format!("{} of {} · {}", board.rows.len(), board.ranked, total);

    let title = format!("Top by {}", metric.label());
    let max = board.max;
    let sum = board.total;

    rsx! {
        div { class: "panel top-consumers",
            div { class: "panel-head",
                h3 { "{title}" }
            }

            div { class: "tc-summary", "{summary}" }

            if board.rows.is_empty() {
                div { class: "tc-empty", "no containers to rank" }
            } else {
                div { class: "tc-body",
                    for (idx , row) in board.rows.into_iter().enumerate() {
                        TopRow {
                            key: "{row.id}",
                            row,
                            rank: idx + 1,
                            max,
                            total: sum,
                            metric,
                            single_vm_name: single_vm_name.clone(),
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TopRow(
    row: TopConsumerRow,
    rank: usize,
    max: f64,
    total: f64,
    metric: TopMetric,
    single_vm_name: Option<String>,
) -> Element {
    // Effective VM of this row: per-row vm in /all view, the selected VM in
    // single-VM view (where row.vm is None).
    let row_vm = row.vm.clone().or_else(|| single_vm_name.clone());

    let (value, unit) = match metric {
        TopMetric::Cpu => (format!("{:.2}", row.cpu), "%".to_string()),
        TopMetric::Mem => {
            let (v, u) = fmt_mem_pair(row.mem_bytes);
            (v, u.to_string())
        }
    };

    let mem_pct = row.mem_pct();

    // What the percentage is measured AGAINST differs by metric, while the
    // ordering never does — both boards rank on absolute consumption.
    //
    // CPU has no per-container ceiling to speak of, so its share of the ranked
    // set is the useful figure. Memory does: the container was given a limit (or,
    // unlimited, may claim the whole host), and "87% of what it may use" answers
    // the question a share of the fleet total cannot. The bar tracks whichever
    // percentage the row is labelled with, so the two never disagree.
    let (share, bar_pct) = match metric {
        TopMetric::Cpu => (
            row.share_pct(total)
                .map(|p| format!("{:.1}% of total", p))
                .unwrap_or_else(|| "idle".to_string()),
            row.bar_pct(max),
        ),
        TopMetric::Mem => match mem_pct {
            Some(pct) => (
                format!(
                    "{:.1}% of {}",
                    pct,
                    if row.mem_limit_is_declared {
                        "limit"
                    } else {
                        "host RAM"
                    }
                ),
                pct.clamp(0.0, 100.0),
            ),
            // No declared limit and no host RAM reading: nothing to be a
            // percentage OF, so fall back to the leader-relative bar.
            None => ("no limit known".to_string(), row.bar_pct(max)),
        },
    };

    // Memory heat mirrors the container list so a hot row reads the same in
    // both columns — but only on the board that ranks by memory.
    let heat = match (metric, mem_pct) {
        (TopMetric::Mem, Some(p)) if p >= 90.0 => " mem-danger",
        (TopMetric::Mem, Some(p)) if p >= 80.0 => " mem-warn",
        _ => "",
    };
    let row_class = format!("tc-row{}", heat);
    let state_cls = format!("state {}", row.state_class);
    let bar_width = (bar_pct * 10.0).round() / 10.0;
    let fill_class = format!("tc-fill {}", metric.fill_class());
    let bar_title = match (metric, mem_pct, row.mem_limit_is_declared) {
        (TopMetric::Mem, Some(p), true) => format!("{:.0}% of declared mem limit", p),
        (TopMetric::Mem, Some(p), false) => format!("{:.0}% of host RAM (no container limit)", p),
        _ => format!("{} {} — {}", value, unit, share),
    };

    let target = match (&single_vm_name, &row.vm) {
        (Some(vm), _) => AppRoute::ContainerRoute {
            vm_name: vm.clone(),
            container_name: row.name.clone(),
        },
        (None, Some(rv)) => AppRoute::AllContainerRoute {
            vm_name: rv.clone(),
            container_name: row.name.clone(),
        },
        // /all view but no per-row vm (shouldn't happen with current server) — degrade to Home.
        (None, None) => AppRoute::AllRoute {},
    };

    rsx! {
        Link { to: target, class: "{row_class}",
            span { class: "rank", "{rank}" }
            span { class: "{state_cls}" }
            div { class: "info",
                div { class: "name",
                    "{row.name}"
                    if single_vm_name.is_none() {
                        if let Some(vm) = row_vm.as_ref() {
                            span { class: "vm", "{vm}" }
                        }
                    }
                }
                div { class: "image", "{row.image}" }
                div { class: "tc-bar", title: "{bar_title}",
                    div {
                        class: "{fill_class}",
                        // Width ONLY — the colour is the class's. See `fill_class`.
                        style: "width: {bar_width:.1}%;",
                    }
                }
            }
            div { class: "tc-val",
                span { class: "v",
                    "{value}"
                    span { class: "u", "{unit}" }
                }
                span { class: "sub", "{share}" }
            }
        }
    }
}
