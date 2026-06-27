use std::rc::Rc;

use dioxus::prelude::*;

use crate::states::MainState;
use crate::views::dockerscope::charts::AreaChart;
use crate::views::dockerscope::detail::*;
use crate::views::dockerscope::helpers::*;

#[component]
pub fn DetailPanel(env: Rc<String>) -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let cs_ra = main_state.read();

    let Some(item) = cs_ra.find_active_container() else {
        return rsx! {
            main { class: "detail",
                div { class: "detail-empty", "select a container" }
            }
        };
    };

    let container = item.container.clone();
    let vm_url = item.url.clone();
    // VM name for the hero header: per-row vm in /all view, falls back to the
    // routed VM in single-VM view (where item.vm is None).
    let vm_name = item
        .vm
        .clone()
        .or_else(|| cs_ra.get_active_container_vm().map(|v| v.to_string()))
        .unwrap_or_default();
    let _ = env;

    let mem_limit = container.mem.limit.unwrap_or(0);
    let mem_now = container.mem.usage.unwrap_or(0);
    let cpu_history = container
        .cpu_usage_history
        .clone()
        .unwrap_or_else(|| vec![container.cpu.usage.unwrap_or(0.0)]);
    let mem_history_bytes = container
        .mem_usage_history
        .clone()
        .unwrap_or_else(|| vec![mem_now]);
    let mem_history_mib: Vec<f64> = mem_history_bytes
        .iter()
        .map(|b| (*b as f64) / (1024.0 * 1024.0))
        .collect();

    let cpu_now = cpu_history.last().copied().unwrap_or(0.0);
    let cpu_prev = cpu_history
        .iter()
        .rev()
        .nth(5)
        .copied()
        .unwrap_or(cpu_now);
    let cpu_delta = cpu_now - cpu_prev;

    let (mem_v, mem_u) = fmt_mem_pair(mem_now);
    let (lim_v, lim_u) = fmt_mem_pair(mem_limit);
    let mem_pct_now = pct(mem_now, mem_limit) as i32;

    let net_in_history = container
        .net_in_history
        .clone()
        .unwrap_or_else(|| vec![container.net.in_mbps.unwrap_or(0.0)]);
    let net_out_history = container
        .net_out_history
        .clone()
        .unwrap_or_else(|| vec![container.net.out_mbps.unwrap_or(0.0)]);
    let net_in_now = container
        .net
        .in_mbps
        .or_else(|| net_in_history.last().copied())
        .unwrap_or(0.0);
    let net_out_now = container
        .net
        .out_mbps
        .or_else(|| net_out_history.last().copied())
        .unwrap_or(0.0);

    let (net_in_v, net_in_u) = fmt_throughput_pair(net_in_now);
    let (net_out_v, net_out_u) = fmt_throughput_pair(net_out_now);

    rsx! {
        main { class: "detail",
            Hero { container: container.clone(), vm_url: vm_url.clone(), vm_name }

            div { class: "charts-row",
                ChartCard {
                    label: "CPU".to_string(),
                    color: "#4ade80".to_string(),
                    big_value: format!("{:.2}", cpu_now),
                    unit: "%".to_string(),
                    delta_value: format!("{:.2}%", cpu_delta.abs()),
                    delta_up: cpu_delta >= 0.0,
                    sub: "2s window".to_string(),
                    values: cpu_history,
                }
                ChartCard {
                    label: "Memory".to_string(),
                    color: "#60a5fa".to_string(),
                    big_value: mem_v.clone(),
                    unit: mem_u.to_string(),
                    delta_value: format!("of {} {}", lim_v, lim_u),
                    delta_up: true,
                    sub: format!("limit {} {} · {}% used", lim_v, lim_u, mem_pct_now),
                    values: mem_history_mib,
                }
                ChartCard {
                    label: "Net In".to_string(),
                    color: "#a78bfa".to_string(),
                    big_value: net_in_v,
                    unit: net_in_u,
                    delta_value: "↓ inbound".to_string(),
                    delta_up: true,
                    sub: "2s window".to_string(),
                    values: net_in_history,
                }
                ChartCard {
                    label: "Net Out".to_string(),
                    color: "#f59e0b".to_string(),
                    big_value: net_out_v,
                    unit: net_out_u,
                    delta_value: "↑ outbound".to_string(),
                    delta_up: true,
                    sub: "2s window".to_string(),
                    values: net_out_history,
                }
            }

            StatLine { container: container.clone() }

            div { class: "detail-bottom",
                div { class: "ports-mounts-row",
                    PortsPanel { ports: container.ports.clone() }
                    MountsPanel { volumes: container.volumes.clone() }
                }
                LogPreview {
                    // Stable key per container — without it Dioxus may remount
                    // the component on every tick the polling loop updates
                    // MainState, which would tear down the WebSocket and
                    // reopen it. Keying by container.id pins identity.
                    key: "{container.id}",
                    container_id: container.id.clone(),
                    vm_url: vm_url.clone(),
                    is_running: container.state.as_deref() == Some("running"),
                }
                ComposePanel { container: container.clone() }
                LabelsPanel { container: container.clone() }
            }
        }
    }
}

#[component]
fn ChartCard(
    label: String,
    color: String,
    big_value: String,
    unit: String,
    delta_value: String,
    delta_up: bool,
    sub: String,
    values: Vec<f64>,
) -> Element {
    let delta_class = if delta_up { "delta up" } else { "delta down" };
    let arrow = if delta_up { "▲" } else { "▼" };
    let color_for_chart = color.clone();
    rsx! {
        div { class: "chart-card",
            div { class: "head",
                span { class: "label",
                    span { class: "sw", style: "background: {color};" }
                    "{label}"
                }
                span { class: "sub", "{sub}" }
            }
            div { class: "value-row",
                span { class: "big",
                    "{big_value}"
                    span { class: "unit", "{unit}" }
                }
                span { class: "{delta_class}", "{arrow} {delta_value}" }
            }
            AreaChart { values, color: color_for_chart, height: 92, unit: "".to_string() }
        }
    }
}
