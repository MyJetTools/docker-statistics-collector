use dioxus::prelude::*;

use crate::states::{ContainerFilter, MainState};
use crate::views::dockerscope::helpers::*;
use crate::views::dockerscope::icons::*;

#[component]
pub fn ContainerListPanel() -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let cs_ra = main_state.read();

    let title = cs_ra
        .get_selected_vm_name()
        .unwrap_or_else(|| "no vm".to_string());

    let total_count = cs_ra.get_all_containers().map(|c| c.len()).unwrap_or(0);

    let Some(rows_src) = cs_ra.get_containers() else {
        return rsx! {
            section { class: "list-col",
                ListHead { title, total: total_count }
                div { class: "list-body",
                    div { class: "ds-loading", "no vm selected" }
                }
            }
        };
    };

    let active_id = cs_ra.get_active_container_id().map(|s| s.to_string());
    let rows: Vec<_> = rows_src
        .iter()
        .map(|m| ContainerRowData::from(*m))
        .collect();

    rsx! {
        section { class: "list-col",
            ListHead { title, total: total_count }
            div { class: "list-body",
                if rows.is_empty() {
                    div {
                        style: "padding:40px 12px;text-align:center;font-family:var(--mono);font-size:11.5px;color:var(--text-muted);",
                        "no containers match"
                    }
                } else {
                    for row in rows.into_iter() {
                        ContainerRow { row, active_id: active_id.clone() }
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Clone)]
struct ContainerRowData {
    id: String,
    name: String,
    image: String,
    state_class: &'static str,
    cpu: f64,
    mem_bytes: i64,
}

impl ContainerRowData {
    fn from(m: &crate::models::MetricsByVm) -> Self {
        let c = &m.container;
        let name = c
            .names
            .first()
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| shorten_id(&c.id, 12).to_string());
        Self {
            id: c.id.clone(),
            name,
            image: c.image.clone(),
            state_class: state_class_for(c.state.as_deref()),
            cpu: c.cpu.usage.unwrap_or(0.0),
            mem_bytes: c.mem.usage.unwrap_or(0),
        }
    }
}

#[component]
fn ContainerRow(row: ContainerRowData, active_id: Option<String>) -> Element {
    let is_active = active_id.as_deref() == Some(row.id.as_str());
    let row_class = if is_active {
        "cont-row active"
    } else {
        "cont-row"
    };
    let state_cls = format!("state {}", row.state_class);
    let id_for_click = row.id.clone();
    let cpu_str = format!("{:.2}%", row.cpu);
    let mem_str = fmt_mem_short(row.mem_bytes);

    rsx! {
        div {
            class: "{row_class}",
            onclick: move |_| {
                consume_context::<Signal<MainState>>()
                    .write()
                    .set_active_container_id(Some(id_for_click.clone()));
            },
            span { class: "{state_cls}" }
            div { class: "info",
                div { class: "name", "{row.name}" }
                div { class: "image", "{row.image}" }
            }
            div { class: "metrics",
                span { class: "cpu", "{cpu_str}" }
                span { class: "mem", "{mem_str}" }
            }
        }
    }
}

#[component]
fn ListHead(title: String, total: usize) -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let cs_ra = main_state.read();
    let query = cs_ra.get_filter().to_string();
    let active_filter = cs_ra.get_container_filter();

    // Filter counts — read from full unfiltered set so the chip badges are stable.
    let mut all = 0;
    let mut running = 0;
    let mut exited = 0;
    let mut restarting = 0;
    let mut unhealthy = 0;
    if let Some(items) = cs_ra.get_all_containers() {
        for itm in items {
            all += 1;
            let s = itm
                .container
                .state
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase();
            if s == "running" {
                running += 1;
            } else if s == "restarting" {
                restarting += 1;
            } else if s.contains("unhealthy") {
                unhealthy += 1;
            } else {
                exited += 1;
            }
        }
    }
    drop(cs_ra);

    rsx! {
        div { class: "list-head",
            div { class: "title-row",
                h2 { "{title}" }
                span { class: "sub", "{total} containers" }
            }
            div { class: "search",
                {icon_search()}
                input {
                    placeholder: "filter by name, image, id…",
                    value: "{query}",
                    oninput: move |evt| {
                        consume_context::<Signal<MainState>>()
                            .write()
                            .set_filter(evt.value());
                    },
                }
                span { class: "kbd", "⌘K" }
            }
            div { class: "filters",
                FilterChip {
                    label: "all", filter: ContainerFilter::All,
                    count: all, active: active_filter == ContainerFilter::All,
                }
                FilterChip {
                    label: "running", filter: ContainerFilter::Running,
                    count: running, active: active_filter == ContainerFilter::Running,
                }
                FilterChip {
                    label: "unhealthy", filter: ContainerFilter::Unhealthy,
                    count: unhealthy, active: active_filter == ContainerFilter::Unhealthy,
                }
                FilterChip {
                    label: "restarting", filter: ContainerFilter::Restarting,
                    count: restarting, active: active_filter == ContainerFilter::Restarting,
                }
                FilterChip {
                    label: "exited", filter: ContainerFilter::Exited,
                    count: exited, active: active_filter == ContainerFilter::Exited,
                }
            }
        }
    }
}

#[component]
fn FilterChip(label: String, filter: ContainerFilter, count: usize, active: bool) -> Element {
    let chip_class = if active { "chip active" } else { "chip" };
    let dot_color = match filter {
        ContainerFilter::Running => "var(--accent)",
        ContainerFilter::Exited => "var(--text-muted)",
        ContainerFilter::Restarting => "var(--warn)",
        ContainerFilter::Unhealthy => "var(--danger)",
        ContainerFilter::All => "var(--text-dim)",
    };

    rsx! {
        button {
            class: "{chip_class}",
            onclick: move |_| {
                consume_context::<Signal<MainState>>()
                    .write()
                    .set_container_filter(filter);
            },
            span { class: "dot", style: "background: {dot_color};" }
            "{label}"
            span { style: "color: var(--text-muted); margin-left: 2px;", " {count}" }
        }
    }
}
