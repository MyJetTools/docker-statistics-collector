use dioxus::prelude::*;

use crate::router::AppRoute;
use crate::states::{MainState, TopMetric, TOP_N_OPTIONS};
use crate::views::dockerscope::detail::{build_top_consumers, TopConsumerRow};
use crate::views::dockerscope::helpers::{fmt_mem_pair, fmt_mem_short};
use crate::views::dockerscope::icons::*;

/// Fills the detail column while a VM is selected but no container is: the
/// heaviest containers of that VM ranked by CPU or memory, Top-N selectable.
#[component]
pub fn TopConsumersPanel() -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let cs_ra = main_state.read();

    let all_selected = cs_ra.is_all_vms_selected();
    let scope_title = if all_selected {
        "All VMs".to_string()
    } else {
        cs_ra
            .get_selected_vm_name()
            .unwrap_or_else(|| "no vm".to_string())
    };
    let single_vm_name = if all_selected {
        None
    } else {
        cs_ra.get_selected_vm_name()
    };

    let metric = cs_ra.get_top_metric();
    let top_n = cs_ra.get_top_n();
    let board = build_top_consumers(&cs_ra.get_containers(), metric, top_n);

    let (total_mem_v, total_mem_u) = fmt_mem_pair(board.total_mem);
    let shown = board.rows.len();
    let summary = format!(
        "{} of {} containers · CPU {:.2}% · MEM {} {}",
        shown, board.ranked, board.total_cpu, total_mem_v, total_mem_u
    );

    let max = board.max;
    let total = board.total;

    rsx! {
        div { class: "panel top-consumers",
            div { class: "panel-head",
                h3 { "Top consumers · {scope_title}" }
                div { class: "tc-controls",
                    select {
                        class: "tc-select",
                        title: "rank containers by",
                        oninput: move |evt| {
                            consume_context::<Signal<MainState>>()
                                .write()
                                .set_top_metric(TopMetric::parse(&evt.value()));
                        },
                        for m in [TopMetric::Cpu, TopMetric::Mem] {
                            option {
                                value: "{m.as_key()}",
                                selected: m == metric,
                                "{m.label()}"
                            }
                        }
                    }
                    select {
                        class: "tc-select",
                        title: "how many containers to show",
                        oninput: move |evt| {
                            let value = evt.value().parse::<usize>().unwrap_or(0);
                            consume_context::<Signal<MainState>>().write().set_top_n(value);
                        },
                        for n in TOP_N_OPTIONS {
                            option {
                                value: "{n}",
                                selected: n == top_n,
                                {top_n_label(n)}
                            }
                        }
                    }
                }
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
                            total,
                            metric,
                            single_vm_name: single_vm_name.clone(),
                        }
                    }
                }
            }
        }
    }
}

fn top_n_label(n: usize) -> String {
    if n == 0 {
        "All".to_string()
    } else {
        format!("Top {}", n)
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

    let share = row
        .share_pct(total)
        .map(|p| format!("{:.1}% of total", p))
        .unwrap_or_else(|| "idle".to_string());

    let mem_pct = row.mem_pct();
    let (alt, alt_icon) = match metric {
        TopMetric::Cpu => {
            let text = match row.effective_mem_limit {
                Some(limit) => format!(
                    "{} / {}",
                    fmt_mem_short(row.mem_bytes),
                    fmt_mem_short(limit)
                ),
                None => fmt_mem_short(row.mem_bytes),
            };
            (text, icon_memory())
        }
        TopMetric::Mem => (format!("{:.2}%", row.cpu), icon_cpu()),
    };

    // Memory heat mirrors the container list so a hot row reads the same in
    // both columns — but only when memory is what's being ranked.
    let heat = match (metric, mem_pct) {
        (TopMetric::Mem, Some(p)) if p >= 90.0 => " mem-danger",
        (TopMetric::Mem, Some(p)) if p >= 80.0 => " mem-warn",
        _ => "",
    };
    let row_class = format!("tc-row{}", heat);
    let state_cls = format!("state {}", row.state_class);
    let bar_width = (row.bar_pct(max) * 10.0).round() / 10.0;
    let bar_color = metric.color_var();
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
                        class: "tc-fill",
                        style: "width: {bar_width:.1}%; background: {bar_color};",
                    }
                }
            }
            div { class: "tc-val",
                span { class: "v",
                    "{value}"
                    span { class: "u", "{unit}" }
                }
                span { class: "sub", "{share}" }
                span { class: "alt",
                    span { class: "aico", {alt_icon} }
                    "{alt}"
                }
            }
        }
    }
}
