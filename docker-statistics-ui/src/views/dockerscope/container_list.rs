use dioxus::prelude::*;

use crate::router::AppRoute;
use crate::states::{primary_name, ContainerFilter, MainState};
use crate::views::dockerscope::helpers::*;
use crate::views::dockerscope::icons::*;

#[component]
pub fn ContainerListPanel() -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let cs_ra = main_state.read();

    let all_selected = cs_ra.is_all_vms_selected();
    let title = if all_selected {
        "All VMs".to_string()
    } else {
        cs_ra
            .get_selected_vm_name()
            .unwrap_or_else(|| "no vm".to_string())
    };

    let total_count = cs_ra.get_all_containers().len();
    let rows_src = cs_ra.get_containers();

    let active_name = cs_ra.get_active_container_name().map(|s| s.to_string());
    let active_vm = cs_ra.get_active_container_vm().map(|s| s.to_string());
    let single_vm_name = if all_selected {
        None
    } else {
        cs_ra.get_selected_vm_name()
    };
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
                        ContainerRow {
                            // Stable key per container — without it Dioxus may
                            // remount the row on every tick (when the polling
                            // loop replaces the Vec), wiping `use_signal` state
                            // in MemBar and making the bar flicker when pct
                            // momentarily goes None.
                            key: "{row.id}",
                            row: row.clone(),
                            active_name: active_name.clone(),
                            active_vm: active_vm.clone(),
                            single_vm_name: single_vm_name.clone(),
                        }
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
    /// Source VM — `Some` in `/all` view (carried over from `MetricsByVm.vm`),
    /// `None` when the container list is scoped to a single VM (the VM is
    /// implicit and lives in the URL prefix).
    vm: Option<String>,
    image: String,
    state_class: &'static str,
    is_running: bool,
    cpu: f64,
    mem_bytes: i64,
    /// Effective memory limit in bytes: declared limit if set, otherwise host
    /// total RAM (an unlimited container can claim the whole VM). `None` when
    /// neither is known.
    effective_mem_limit: Option<i64>,
    /// Whether `effective_mem_limit` came from `mem.limit` (true) or fell back
    /// to host RAM (false). Drives the tooltip wording.
    mem_limit_is_declared: bool,
    net_in_mbps: f64,
    net_out_mbps: f64,
}

impl ContainerRowData {
    fn from(m: &crate::models::MetricsByVm) -> Self {
        let c = &m.container;
        let name = if c.names.is_empty() {
            shorten_id(&c.id, 12).to_string()
        } else {
            primary_name(&c.names).to_string()
        };
        let mem_used = c.mem.usage.unwrap_or(0);
        let (effective_mem_limit, mem_limit_is_declared) = match c.mem.limit {
            Some(v) if v > 0 => (Some(v), true),
            _ => (m.host_mem_total, false),
        };
        Self {
            id: c.id.clone(),
            name,
            vm: m.vm.clone(),
            image: c.image.clone(),
            state_class: state_class_for(c.state.as_deref()),
            is_running: c.state.as_deref() == Some("running"),
            cpu: c.cpu.usage.unwrap_or(0.0),
            mem_bytes: mem_used,
            effective_mem_limit,
            mem_limit_is_declared,
            net_in_mbps: c.net.in_mbps.unwrap_or(0.0),
            net_out_mbps: c.net.out_mbps.unwrap_or(0.0),
        }
    }

    fn mem_pct(&self) -> Option<f64> {
        let limit = self.effective_mem_limit?;
        if limit <= 0 {
            return None;
        }
        Some((self.mem_bytes as f64 / limit as f64) * 100.0)
    }
}

#[component]
fn ContainerRow(
    row: ContainerRowData,
    active_name: Option<String>,
    active_vm: Option<String>,
    single_vm_name: Option<String>,
) -> Element {
    // Effective VM of this row: comes from MetricsByVm.vm in /all view, falls
    // back to the currently selected VM in single-VM view (where row.vm is None).
    let row_vm = row.vm.clone().or_else(|| single_vm_name.clone());
    let is_active = active_name
        .as_deref()
        .map(|n| n.eq_ignore_ascii_case(&row.name))
        .unwrap_or(false)
        && active_vm.as_deref() == row_vm.as_deref();
    let pct = row.mem_pct();

    let mem_heat = match pct {
        Some(p) if p >= 90.0 => " mem-danger",
        Some(p) if p >= 80.0 => " mem-warn",
        _ => "",
    };
    let active_cls = if is_active { " active" } else { "" };
    let row_class = format!("cont-row{}{}", active_cls, mem_heat);
    let state_cls = format!("state {}", row.state_class);
    let cpu_str = format!("{:.2}%", row.cpu);

    let used_str = fmt_mem_short(row.mem_bytes);
    let limit_str = row.effective_mem_limit.map(fmt_mem_short);
    let mem_title = match (pct, row.mem_limit_is_declared) {
        (Some(p), true) => format!("{:.0}% of declared mem limit", p),
        (Some(p), false) => format!("{:.0}% of host RAM (no container limit)", p),
        (None, _) => "no mem limit known".to_string(),
    };
    let badge = pct.filter(|p| *p >= 80.0).map(|p| (p, p >= 90.0));

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
        Link {
            to: target,
            class: "{row_class}",
            span { class: "{state_cls}" }
            div { class: "info",
                div { class: "name",
                    "{row.name}"
                    if let Some((p, crit)) = badge {
                        MemBadge { pct: p, danger: crit }
                    }
                }
                div { class: "image", "{row.image}" }
                MemBar { pct, running: row.is_running }
            }
            div { class: "metrics",
                span { class: "cpu", "{cpu_str}" }
                span { class: "mem", title: "{mem_title}",
                    "{used_str}"
                    if let Some(lim) = limit_str.as_ref() {
                        span { style: "color: var(--text-muted);", " / {lim}" }
                    }
                }
                span {
                    class: "net",
                    title: "network in / out (MB/s)",
                    "↓{row.net_in_mbps:.2} ↑{row.net_out_mbps:.2}"
                }
            }
        }
    }
}

#[component]
fn MemBar(pct: Option<f64>, running: bool) -> Element {
    if !running {
        return rsx! {};
    }

    // Sticky last-known pct: backend occasionally drops `mem.limit` for a
    // single tick (peer fanout race) or reports mem.usage=0 on the hottest
    // containers (kafka, loggers). Hold the previous value and only fall back
    // to it when the current pct is None or 0 — otherwise the bar disappears
    // every few seconds exactly on the busiest rows.
    let mut last_pct = use_signal::<Option<f64>>(|| None);
    let incoming = pct;
    use_effect(use_reactive!(|incoming| {
        if let Some(v) = incoming {
            if v > 0.0 {
                last_pct.set(Some(v));
            }
        }
    }));

    let effective = pct.filter(|p| *p > 0.0).or(*last_pct.read());
    let Some(p) = effective else {
        return rsx! {};
    };
    if p <= 0.0 {
        return rsx! {};
    }

    // Colour + glow live in CSS classes (.mb-mem/.mb-warn/.mb-danger), not in
    // the inline style. Inline style only carries width — see dockerscope.css.
    // Round width to 1 decimal so the inline string stays stable when pct
    // wobbles in the noise digits between polling ticks.
    let mode = if p >= 90.0 {
        "mb-danger"
    } else if p >= 80.0 {
        "mb-warn"
    } else {
        ""
    };
    let width = (p.min(100.0) * 10.0).round() / 10.0;
    rsx! {
        div { class: "mb-outer {mode}",
            div { class: "mb-fill", style: "width: {width:.1}%;" }
            div { class: "mb-tick" }
        }
    }
}

#[component]
fn MemBadge(pct: f64, danger: bool) -> Element {
    let cls = if danger { "mem-badge danger" } else { "mem-badge" };
    let label = format!("⚠ MEM {:.0}%", pct);
    rsx! { span { class: "{cls}", "{label}" } }
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
    for itm in cs_ra.get_all_containers() {
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
                        let value = evt.value();
                        crate::utils::set_url_query("service", &value);
                        consume_context::<Signal<MainState>>()
                            .write()
                            .set_filter(value);
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
