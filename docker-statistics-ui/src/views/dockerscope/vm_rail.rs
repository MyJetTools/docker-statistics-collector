use dioxus::prelude::*;

use crate::router::AppRoute;
use crate::states::MainState;
use crate::views::dockerscope::helpers::*;
use crate::views::dockerscope::icons::*;

/// One pre-rendered host-disk row for a VM card (left rail).
struct DiskRow {
    mount_point: String,
    device: String,
    fs_type: String,
    amount: String,
    pct: f64,
    color_cls: &'static str,
}

#[component]
pub fn VmRail() -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let cs_ra = main_state.read();

    let grouped = group_vms(&cs_ra.vms_state);
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
            // All VMs entry — synthetic aggregate over the whole fleet,
            // rendered through the same VmCard layout for visual parity.
            if !cs_ra.vms_state.is_empty() {
                div { style: "margin-top: 12px;",
                    VmCard {
                        name: "All VMs".to_string(),
                        vm: aggregate_all_vms(&cs_ra.vms_state),
                        active: all_selected,
                        is_all: true,
                    }
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
fn VmCard(
    name: String,
    vm: crate::models::VmModel,
    active: bool,
    #[props(default = false)]
    is_all: bool,
) -> Element {
    let status = vm_status(&vm);
    let heart_class = format!("heart {}", if status == "ok" { "" } else { status });
    let cpu_pct = vm.cpu;
    let card_class = if active { "vm-card active" } else { "vm-card" };
    let target = if is_all {
        AppRoute::AllRoute {}
    } else {
        AppRoute::VmRoute { vm_name: name.clone() }
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
    // Colour comes from a CSS modifier class (not inline) so the inline
    // `style` only carries `width:` — see dockerscope.css for the rules.
    let used_color_cls = match severity {
        MemSeverity::Danger => "col-danger",
        MemSeverity::Warn => "col-warn",
        MemSeverity::Ok => "",
    };
    let reserved_cls = if over_commit { "over-commit" } else { "" };
    let denom_label = host_total_short.clone().unwrap_or_else(|| reserved_short.clone());

    let net_in_str = fmt_throughput(vm.net_in_mbps);
    let net_out_str = fmt_throughput(vm.net_out_mbps);

    // Host physical disks — shown under each real VM, never on the "All VMs"
    // aggregate (disk usage doesn't sum meaningfully across hosts).
    let disk_rows: Vec<DiskRow> = if is_all {
        Vec::new()
    } else {
        vm.host_disks
            .as_ref()
            .map(|disks| {
                disks
                    .iter()
                    .map(|d| {
                        let pct = if d.total > 0 {
                            (d.used as f64 / d.total as f64 * 100.0).clamp(0.0, 100.0)
                        } else {
                            0.0
                        };
                        DiskRow {
                            mount_point: d.mount_point.clone(),
                            device: d.device.clone(),
                            fs_type: d.fs_type.clone(),
                            amount: format!("{} / {}", fmt_mem_short(d.used), fmt_mem_short(d.total)),
                            pct,
                            color_cls: if pct >= 90.0 {
                                "col-danger"
                            } else if pct >= 75.0 {
                                "col-warn"
                            } else {
                                ""
                            },
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    rsx! {
        Link {
            to: target,
            class: "{card_class}",
            title: "{vm.api_url}",
            div { class: "top",
                div { class: "ico",
                    {icon_server()}
                    span { class: "{heart_class}" }
                }
                div { class: "info",
                    div { class: "name", "{name}" }
                }
                div { class: "count", "{vm.containers_amount}" }
            }
            div { class: "meta",
                span { class: "item cpu", "{cpu_pct:.2}% cpu" }
                if let Some(c) = vm.host_cpu_count {
                    span { class: "item", title: "host cores", "{c}c" }
                }
                span {
                    class: "item net",
                    title: "network in / out (sum across containers)",
                    "↓{net_in_str} ↑{net_out_str}"
                }
            }
            div { class: "vm-mem-text",
                span { class: "used", "{used_short}" }
                span { class: "denom", " / {denom_label}" }
                span { class: "sep", " · " }
                span { class: "res-lbl", "res " }
                span { style: "color: {reserved_color};", "{reserved_short}" }
            }
            div { class: "vm-mem-bar", title: "{mem_title}",
                div {
                    class: "vm-mem-bar-reserved {reserved_cls}",
                    style: "width: {reserved_pct:.1}%;",
                }
                div {
                    class: "vm-mem-bar-used {used_color_cls}",
                    style: "width: {used_pct:.1}%;",
                }
                div { class: "vm-mem-bar-tick" }
            }
            if !disk_rows.is_empty() {
                div { class: "vm-disks",
                    for row in disk_rows.iter() {
                        div {
                            class: "vm-disk",
                            title: "{row.device} · {row.fs_type}",
                            div { class: "vm-disk-head",
                                span { class: "mp", "{row.mount_point}" }
                                span { class: "amt", "{row.amount}" }
                            }
                            div { class: "vm-disk-bar",
                                div {
                                    class: "vm-disk-bar-used {row.color_cls}",
                                    style: "width: {row.pct:.1}%;",
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
