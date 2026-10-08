use std::collections::BTreeMap;

use dioxus::prelude::*;

use crate::models::VmModel;
use crate::router::AppRoute;
use crate::views::dockerscope::detail::{build_host_disks, HostDiskRow};
use crate::views::dockerscope::helpers::{fmt_mem_short, DiskSeverity, DISK_WARN_PCT};
use crate::views::dockerscope::icons::icon_disk;

/// The physical disks of the selected host — or of the whole fleet — fullest
/// first. It sits under the container disk board: that one says who is
/// writing, this one says how much room there is left to write into.
///
/// A plain function, like the container boards: the VM map is borrowed straight
/// from the state, and nothing is gained by cloning it into a prop.
pub fn render_host_disks_board(
    vms: &BTreeMap<String, VmModel>,
    single_vm_name: Option<&str>,
    top_n: usize,
) -> Element {
    let board = build_host_disks(vms, single_vm_name, top_n);
    // Scoped to one VM the host is implicit; across the fleet each row names it.
    let fleet_view = single_vm_name.is_none();

    let hot = match board.hot {
        0 => format!("none above {:.0}%", DISK_WARN_PCT),
        hot => format!("{} above {:.0}%", hot, DISK_WARN_PCT),
    };
    let summary = format!("{} of {} · {}", board.rows.len(), board.ranked, hot);
    // A host that reports no disks is not a host with room to spare — name the
    // gap rather than let the board pass for the whole fleet.
    let summary = match board.hosts_without_disks {
        0 => summary,
        1 => format!("{} · 1 VM reports none", summary),
        missing => format!("{} · {} VMs report none", summary, missing),
    };

    rsx! {
        div { class: "panel top-consumers tc-wide",
            div { class: "panel-head",
                h3 { "Host disks" }
                span { class: "tc-hint", "fullest first" }
            }

            if board.rows.is_empty() {
                div {
                    class: "tc-empty",
                    title: "the collector measures host disks through a read-only mount of the host root (-v /:/host/root:ro)",
                    "no host disks reported"
                }
            } else {
                div { class: "tc-summary", "{summary}" }
                div { class: "tc-body",
                    for (idx , row) in board.rows.into_iter().enumerate() {
                        HostDiskRowView {
                            key: "{row.vm}{row.mount_point}",
                            row,
                            rank: idx + 1,
                            fleet_view,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn HostDiskRowView(row: HostDiskRow, rank: usize, fleet_view: bool) -> Element {
    // Heat mirrors the bar, so a filling disk is caught on a scan of the board
    // and not only by reading its number.
    let heat = match row.severity {
        DiskSeverity::Danger => " disk-danger",
        DiskSeverity::Warn => " disk-warn",
        DiskSeverity::Ok => "",
    };
    let row_class = format!("tc-row tc-disk-row{}", heat);
    // A CLASS for the colour, never an inline one — see `TopMetric::fill_class`.
    let fill_class = format!("tc-fill disk {}", row.severity.fill_class());
    // The bar is the disk itself, 0..100 — not scaled against the leader the way
    // the container boards are, because here the headline already is a percentage.
    let bar_width = (row.used_pct * 10.0).round() / 10.0;

    let used_pct = format!("{:.1}", row.used_pct);
    let free = fmt_mem_short(row.available);
    let amount = format!("{} of {}", fmt_mem_short(row.used), fmt_mem_short(row.total));
    let bar_title = format!("{}% used — {} available", used_pct, free);

    let body = rsx! {
        span { class: "rank", "{rank}" }
        span { class: "dico", {icon_disk()} }
        div { class: "info",
            div { class: "name",
                "{row.mount_point}"
                if fleet_view {
                    span { class: "vm", "{row.vm}" }
                }
            }
            div { class: "image", "{row.device} · {row.fs_type} · {free} free" }
            div { class: "tc-bar", title: "{bar_title}",
                div {
                    class: "{fill_class}",
                    // Width ONLY — the colour is the class's.
                    style: "width: {bar_width:.1}%;",
                }
            }
        }
        div { class: "tc-val",
            span { class: "v",
                "{used_pct}"
                span { class: "u", "%" }
            }
            span { class: "sub", "{amount}" }
        }
    };

    // Across the fleet a row leads to its host, where the container disk board
    // then shows who is filling it. Scoped to that host already it would lead
    // nowhere, so there it is not a link.
    if fleet_view {
        rsx! {
            Link {
                to: AppRoute::VmRoute { vm_name: row.vm.clone() },
                class: "{row_class}",
                {body}
            }
        }
    } else {
        rsx! {
            div { class: "{row_class}", {body} }
        }
    }
}
