use dioxus::prelude::*;
use rust_extensions::date_time::DateTimeAsMicroseconds;

/// Format a unix-microseconds-or-seconds timestamp coming from `container.created`
/// (which is stored as seconds in the Docker world; rust-extensions' `from`
/// treats it as seconds via the `i64 -> DateTime` impl).
fn format_ts(c: i64) -> String {
    let t = DateTimeAsMicroseconds::from(c).to_rfc3339();
    t[..19].to_string()
}

/// Same but for unix-seconds (what the collector emits for `started_at`).
fn format_ts_unix_seconds(s: i64) -> String {
    let mut dt = DateTimeAsMicroseconds::new(0);
    dt.unix_microseconds = s * 1_000_000;
    let t = dt.to_rfc3339();
    t[..19].to_string()
}

fn unix_us_to_hours_ago(c: i64) -> f64 {
    let then_us = DateTimeAsMicroseconds::from(c).unix_microseconds;
    let now_us = dioxus_utils::now_date_time().unix_microseconds;
    (now_us - then_us) as f64 / 1_000_000.0 / 3600.0
}

/// `<24h ago` → red bold; anything else → default style.
fn fresh_style(hours_ago: Option<f64>) -> &'static str {
    match hours_ago {
        Some(h) if h >= 0.0 && h < 24.0 => "color: var(--danger); font-weight: 600;",
        _ => "",
    }
}

use crate::models::ContainerModel;
use crate::states::{DialogState, DialogType, MainState};
use crate::views::dockerscope::helpers::shorten_id;
use crate::views::dockerscope::icons::*;

#[component]
pub fn Hero(container: ContainerModel, vm_url: String, vm_name: String) -> Element {
    let state = container.state.clone().unwrap_or_else(|| "—".to_string());
    let lower = state.to_ascii_lowercase();
    let is_running = lower == "running";

    let state_color = if is_running {
        "var(--accent)"
    } else if lower == "restarting" {
        "var(--warn)"
    } else if lower.contains("unhealthy") {
        "var(--danger)"
    } else {
        "var(--text-muted)"
    };
    let bg = if is_running {
        "var(--accent-soft)"
    } else {
        "rgba(255,255,255,.04)"
    };
    let border = if is_running {
        "rgba(74,222,128,.3)"
    } else {
        "var(--border)"
    };

    let name = container
        .names
        .first()
        .map(|n| n.trim_start_matches('/').to_string())
        .unwrap_or_else(|| "unnamed".to_string());

    let uptime = container
        .created
        .map(|c| {
            let then = DateTimeAsMicroseconds::from(c);
            let now = dioxus_utils::now_date_time();
            now.duration_since(then).to_string()
        })
        .unwrap_or_else(|| "—".to_string());

    let id_short = shorten_id(&container.id, 12).to_string();
    let id_full = container.id.clone();
    let image = container.image.clone();

    let dialog_handle = consume_context::<Signal<DialogState>>();
    let id_for_logs = container.id.clone();
    let url_for_logs = vm_url.clone();
    let image_for_logs = image.clone();

    let id_for_procs = container.id.clone();
    let url_for_procs = vm_url.clone();
    let image_for_procs = image.clone();

    let env = consume_context::<Signal<MainState>>()
        .read()
        .envs
        .get_selected_env()
        .map(|e| e.clone())
        .unwrap_or_else(|| std::rc::Rc::new(String::new()));
    let env_logs = env.clone();
    let env_procs = env.clone();

    let stack = container
        .labels
        .as_ref()
        .and_then(|l| l.get("com.docker.compose.project"))
        .cloned()
        .unwrap_or_else(|| "—".to_string());
    // Prefix with the VM so the header reads `{vm}/{stack}` — which machine
    // the stack runs on is the first thing one wants to know here.
    let stack = if vm_name.is_empty() {
        stack
    } else {
        format!("{}/{}", vm_name, stack)
    };

    // "created" — when the container record was created (docker create).
    // "started" — when the main process was last started (changes on restart).
    // Each gets independent <24h-red treatment so a recent restart shows up
    // even when the container itself is old, and a fresh deploy shows up even
    // when it hasn't been restarted yet.
    let created_str = container
        .created
        .map(format_ts)
        .unwrap_or_else(|| "—".to_string());
    let started_str = container
        .started_at
        .map(format_ts_unix_seconds)
        .unwrap_or_else(|| "—".to_string());

    let created_style = fresh_style(container.created.map(unix_us_to_hours_ago));
    let started_style = fresh_style(
        container
            .started_at
            .map(|s| (dioxus_utils::now_date_time().unix_microseconds / 1_000_000 - s) as f64 / 3600.0),
    );

    rsx! {
        div { class: "hero",
            div { class: "top-row",
                span {
                    class: "state-pill",
                    style: "color: {state_color}; background: {bg}; border-color: {border};",
                    span { class: "dot" }
                    "{lower}"
                }
                span { class: "uptime", "up {uptime}" }
                div { class: "actions",
                    button {
                        class: "btn",
                        onclick: move |_| {
                            dialog_handle.clone().write().show_dialog(
                                format!("Logs of {}", image_for_logs),
                                DialogType::ShowLogs {
                                    env: env_logs.clone(),
                                    url: url_for_logs.clone(),
                                    container_id: id_for_logs.clone(),
                                },
                            );
                        },
                        {icon_logs()} " logs"
                    }
                    button {
                        class: "btn",
                        onclick: move |_| {
                            dialog_handle.clone().write().show_dialog(
                                format!("Processes of {}", image_for_procs),
                                DialogType::ShowProcesses {
                                    env: env_procs.clone(),
                                    url: url_for_procs.clone(),
                                    container_id: id_for_procs.clone(),
                                },
                            );
                        },
                        {icon_terminal()} " procs"
                    }
                    button { class: "btn",
                        if is_running { {icon_pause()} " stop" } else { {icon_play()} " start" }
                    }
                    button { class: "btn", {icon_refresh()} " restart" }
                    button { class: "btn", {icon_more()} }
                }
            }
            h1 { "{name}" }
            div { class: "subline",
                span {
                    class: "id-mono",
                    title: "{id_full}",
                    "{id_short}"
                    {icon_copy()}
                }
                span { class: "sep", "·" }
                span { class: "img-tag", "{image}" }
                span { class: "sep", "·" }
                span { span { class: "k", "stack" } " {stack}" }
                span { class: "sep", "·" }
                span {
                    span { class: "k", "created" }
                    " "
                    span { style: "{created_style}", "{created_str}" }
                }
                span { class: "sep", "·" }
                span {
                    span { class: "k", "started" }
                    " "
                    span { style: "{started_style}", "{started_str}" }
                }
            }
        }
    }
}
