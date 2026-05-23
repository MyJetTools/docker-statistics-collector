use dioxus::prelude::*;

use crate::router::AppRoute;
use crate::states::MainState;
use crate::views::dockerscope::helpers::*;
use crate::views::dockerscope::icons::*;

#[component]
pub fn VmRail() -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let cs_ra = main_state.read();

    let Some(vms) = cs_ra.vms_state.as_ref() else {
        return rsx! {
            aside { class: "vm-rail",
                div { class: "ds-loading", "loading vms…" }
            }
        };
    };

    let grouped = group_vms(vms);
    let active_vm = cs_ra.get_selected_vm_name();
    let all_selected = cs_ra.is_all_vms_selected();

    let prod_count = grouped.production.len();
    let stage_count = grouped.staging.len();
    let dev_count = grouped.dev.len();

    rsx! {
        aside { class: "vm-rail",
            VmGroupSection {
                label: "Production".to_string(),
                count: prod_count,
                items: grouped.production.into_iter().map(|(n, v)| (n.clone(), v.clone())).collect::<Vec<_>>(),
                active_vm: active_vm.clone(),
            }
            VmGroupSection {
                label: "Staging".to_string(),
                count: stage_count,
                items: grouped.staging.into_iter().map(|(n, v)| (n.clone(), v.clone())).collect::<Vec<_>>(),
                active_vm: active_vm.clone(),
            }
            VmGroupSection {
                label: "Dev / CI".to_string(),
                count: dev_count,
                items: grouped.dev.into_iter().map(|(n, v)| (n.clone(), v.clone())).collect::<Vec<_>>(),
                active_vm: active_vm.clone(),
            }
            // All VMs entry — selects SelectedVm::All so containers from every VM merge.
            Link {
                to: AppRoute::AllRoute {},
                class: if all_selected { "vm-card active" } else { "vm-card" },
                style: "margin-top: 12px;",
                div { class: "ico", {icon_server()} }
                div { class: "body",
                    div { class: "name", "All VMs" }
                    div { class: "meta", span { class: "item", "aggregate" } }
                }
            }
        }
    }
}

#[component]
fn VmGroupSection(
    label: String,
    count: usize,
    items: Vec<(String, crate::models::VmModel)>,
    active_vm: Option<String>,
) -> Element {
    rsx! {
        div { class: "section-label",
            span { "{label}" }
            span { class: "pill", "{count}" }
        }
        for (name, vm) in items.iter() {
            VmCard {
                name: name.clone(),
                vm: vm.clone(),
                active: active_vm.as_deref() == Some(name.as_str()),
            }
        }
    }
}

#[component]
fn VmCard(name: String, vm: crate::models::VmModel, active: bool) -> Element {
    let status = vm_status(&vm);
    let heart_class = format!("heart {}", if status == "ok" { "" } else { status });
    let cpu_pct = vm.cpu.round() as i32;
    let card_class = if active { "vm-card active" } else { "vm-card" };
    let target = AppRoute::VmRoute {
        vm_name: name.clone(),
    };

    let used_short = fmt_mem_short(vm.mem);
    let reserved_short = fmt_mem_short(vm.mem_limit);
    let host_total_short = vm.host_mem_total.map(fmt_mem_short);

    let severity = vm_mem_severity(vm.mem, vm.mem_limit, vm.host_mem_total);
    let (reserved_color, mem_title) = match severity {
        MemSeverity::Danger => (
            "var(--danger)",
            match vm.host_mem_total {
                Some(t) if vm.mem_limit > t => format!(
                    "Reserved {} exceeds host RAM {} — containers can claim more than this VM has.",
                    fmt_mem_short(vm.mem_limit),
                    fmt_mem_short(t),
                ),
                _ => "Used memory above 90% of host RAM.".to_string(),
            },
        ),
        MemSeverity::Warn => (
            "var(--warn)",
            "Reserved or used memory is close to host RAM capacity.".to_string(),
        ),
        MemSeverity::Ok => ("var(--text-dim)", String::new()),
    };

    // Progress bar geometry: denominator = host RAM if known, else reserved
    // (sum of declared limits). Reserved overlay is rendered translucent behind
    // the used fill; if reserved > host_total it stays clamped to 100% but
    // switches color to danger so over-commit is visible at a glance.
    let bar_denom = vm.host_mem_total.unwrap_or(vm.mem_limit).max(1);
    let used_pct = ((vm.mem as f64 / bar_denom as f64) * 100.0).clamp(0.0, 100.0);
    let reserved_pct = ((vm.mem_limit as f64 / bar_denom as f64) * 100.0).clamp(0.0, 100.0);
    let over_commit = vm
        .host_mem_total
        .map(|t| vm.mem_limit > t)
        .unwrap_or(false);
    let used_color = match severity {
        MemSeverity::Danger => "var(--danger)",
        MemSeverity::Warn => "var(--warn)",
        MemSeverity::Ok => "var(--mem)",
    };
    let reserved_overlay = if over_commit {
        "rgba(239,68,68,.22)"
    } else {
        "rgba(96,165,250,.18)"
    };
    let denom_label = host_total_short.clone().unwrap_or_else(|| reserved_short.clone());

    rsx! {
        Link {
            to: target,
            class: "{card_class}",
            title: "{vm.api_url}",
            div { class: "ico",
                {icon_server()}
                span { class: "{heart_class}" }
            }
            div { class: "body",
                div { class: "name", "{name}" }
                div { class: "meta",
                    span { class: "item cpu", "{cpu_pct}% cpu" }
                    if let Some(c) = vm.host_cpu_count {
                        span { class: "item", title: "host cores", "{c}c" }
                    }
                }
                div { class: "vm-mem-bar", title: "{mem_title}",
                    div {
                        class: "vm-mem-bar-reserved",
                        style: "width: {reserved_pct:.1}%; background: {reserved_overlay};",
                    }
                    div {
                        class: "vm-mem-bar-used",
                        style: "width: {used_pct:.1}%; background: {used_color};",
                    }
                    div { class: "vm-mem-bar-tick" }
                }
                div { class: "vm-mem-text",
                    span { class: "used", "{used_short}" }
                    span { class: "denom", " / {denom_label}" }
                    span { class: "sep", " · " }
                    span { class: "res-lbl", "res " }
                    span { style: "color: {reserved_color};", "{reserved_short}" }
                }
            }
            div { class: "count", "{vm.containers_amount}" }
        }
    }
}
