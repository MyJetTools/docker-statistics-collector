use std::rc::Rc;

use dioxus::prelude::*;
use futures::StreamExt;
use reqwasm::websocket::{futures::WebSocket, Message};
use serde::Deserialize;

use crate::api::{get_base_url, LogLineHttpModel};
use crate::states::{DialogState, DialogType, MainState};

/// Soft cap on how many log lines the preview retains in memory — keeps the
/// virtual DOM from ballooning on chatty containers.
const LINE_BUFFER_CAP: usize = 500;
/// Pseudo-frame `tp` value used for the synthetic disconnect/reconnect markers
/// we inject into the buffer so the user sees the stream timeline rather than
/// silently losing connection.
const MARKER_TP: u8 = 200;

#[derive(Deserialize)]
struct WsLogPayload {
    tp: u8,
    line: String,
}

/// Inline log tail — opens a WebSocket to `/ws/logs?env&id` on the api when
/// the active container changes and renders new lines as they arrive. The
/// stream stays live for as long as this component is mounted.
#[component]
pub fn LogPreview(container_id: String, vm_url: String, is_running: bool) -> Element {
    let env = consume_context::<Signal<MainState>>()
        .read()
        .envs
        .get_selected_env()
        .map(|e| e.clone())
        .unwrap_or_else(|| Rc::new(String::new()));

    let cs = use_signal(LogPreviewState::default);
    // Bumping this re-triggers the use_resource below, even when container_id
    // hasn't changed — that's how the Reconnect button works.
    let reconnect_n = use_signal(|| 0u64);

    let env_for_res = env.clone();
    let container_for_res = container_id.clone();
    let reconnect_dep = *reconnect_n.read();
    let _stream_task = use_resource(use_reactive!(|container_for_res, reconnect_dep| {
        let env = env_for_res.clone();
        let mut cs = cs.to_owned();
        async move {
            // Buffer is only cleared when the user switched to a different
            // container — on a reconnect for the same container we keep the
            // old lines and append a "reconnected" marker so context isn't lost.
            {
                let mut w = cs.write();
                let same_container = w.current_id.as_deref() == Some(container_for_res.as_str());
                if !same_container {
                    w.lines.clear();
                    w.current_id = Some(container_for_res.clone());
                } else if reconnect_dep > 0 {
                    push_marker(&mut w.lines, "── reconnected ──");
                }
                // Every fresh subscribe (new container OR explicit reconnect)
                // re-arms the auto-scroll so the freshest lines snap into view.
                w.did_initial_scroll = false;
                w.live = true;
                w.error = None;
            }
            run_stream(cs, env, container_for_res).await;
        }
    }));

    // After the first line(s) arrive, snap the scroll container to the bottom
    // so the user sees the freshest output immediately. Re-armed on every
    // container switch via `did_initial_scroll = false` above.
    let line_count = cs.read().lines.len();
    let did_scroll = cs.read().did_initial_scroll;
    let mut cs_for_effect = cs.to_owned();
    use_effect(use_reactive!(|line_count, did_scroll| {
        if line_count > 0 && !did_scroll {
            cs_for_effect.write().did_initial_scroll = true;
            let _ = dioxus_utils::eval(
                "requestAnimationFrame(() => { \
                    const el = document.getElementById('log-preview-scroll'); \
                    if (el) el.scrollTop = el.scrollHeight; \
                });",
            );
        }
    }));

    let cs_ra = cs.read();
    let status = if cs_ra.live && is_running {
        "● live"
    } else if cs_ra.live {
        "○ paused"
    } else {
        "⨯ disconnected"
    };
    let status_color = if cs_ra.live && is_running {
        "var(--accent)"
    } else if cs_ra.live {
        "var(--text-muted)"
    } else {
        "var(--danger)"
    };
    let disconnected = !cs_ra.live;

    let body = render_body(&cs_ra);
    drop(cs_ra);

    let cid_for_modal = container_id.clone();
    let url_for_modal = vm_url.clone();
    let env_for_modal = env.clone();
    let dialog_handle = consume_context::<Signal<DialogState>>();

    let cid_for_exec = container_id.clone();
    let url_for_exec = vm_url.clone();
    let env_for_exec = env.clone();
    let dialog_handle_exec = dialog_handle.clone();

    let mut reconnect_signal = reconnect_n.to_owned();

    rsx! {
        div { class: "panel log-panel",
            div { class: "panel-head",
                h3 { "Log tail · live · last {LINE_BUFFER_CAP} lines" }
                div { style: "display:flex; gap:8px; align-items:center;",
                    span { class: "count-pill", style: "color: {status_color};", "{status}" }
                    if disconnected {
                        button {
                            class: "btn btn-sm",
                            onclick: move |_| {
                                *reconnect_signal.write() += 1;
                            },
                            "reconnect"
                        }
                    }
                    button {
                        class: "btn btn-sm",
                        onclick: move |_| {
                            dialog_handle.clone().write().show_dialog(
                                "Logs".to_string(),
                                DialogType::ShowLogs {
                                    env: env_for_modal.clone(),
                                    url: url_for_modal.clone(),
                                    container_id: cid_for_modal.clone(),
                                },
                            );
                        },
                        "open full viewer"
                    }
                    button {
                        class: "btn btn-sm",
                        onclick: move |_| {
                            dialog_handle_exec.clone().write().show_dialog(
                                "Exec console".to_string(),
                                DialogType::ShowExec {
                                    env: env_for_exec.clone(),
                                    url: url_for_exec.clone(),
                                    container_id: cid_for_exec.clone(),
                                },
                            );
                        },
                        "exec »_"
                    }
                }
            }
            div { id: "log-preview-scroll", class: "log-mini log-mini-tall", {body} }
        }
    }
}

fn render_body(cs: &LogPreviewState) -> Element {
    if cs.lines.is_empty() && cs.error.is_none() {
        return rsx! {
            div { style: "padding:6px 0; color:var(--text-muted);", "waiting for log lines…" }
        };
    }
    rsx! {
        for line in cs.lines.iter() {
            LogLine { tp: line.tp as i32, text: line.line.clone() }
        }
        if let Some(err) = cs.error.as_deref() {
            div { class: "ds-error", "log stream error: {err}" }
        }
    }
}

#[component]
fn LogLine(tp: i32, text: String) -> Element {
    let cls = match tp {
        t if t == MARKER_TP as i32 => "log-line lvl-marker",
        0 => "log-line lvl-warn",
        1 => "log-line lvl-info",
        2 => "log-line lvl-err",
        _ => "log-line",
    };
    rsx! { div { class: "{cls}", "{text}" } }
}

fn push_marker(lines: &mut Vec<LogLineHttpModel>, label: &str) {
    lines.push(LogLineHttpModel {
        tp: MARKER_TP,
        line: label.to_string(),
    });
    while lines.len() > LINE_BUFFER_CAP {
        lines.remove(0);
    }
}

async fn run_stream(mut cs: Signal<LogPreviewState>, env: Rc<String>, container_id: String) {
    let ws_url = build_ws_url(env.as_str(), &container_id);
    dioxus_utils::console_log(&format!("[logs-ws] opening url={ws_url}"));

    let ws = match WebSocket::open(&ws_url) {
        Ok(ws) => ws,
        Err(err) => {
            dioxus_utils::console_log(&format!("[logs-ws] open failed: {err:?}"));
            let mut w = cs.write();
            w.live = false;
            push_marker(&mut w.lines, &format!("── disconnected (open failed: {err:?}) ──"));
            return;
        }
    };

    let (_write, mut read) = ws.split();
    dioxus_utils::console_log("[logs-ws] connected, waiting for messages…");

    let mut received = 0u64;
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                received += 1;
                if let Ok(parsed) = serde_json::from_str::<WsLogPayload>(&text) {
                    let mut w = cs.write();
                    w.lines.push(LogLineHttpModel {
                        tp: parsed.tp,
                        line: parsed.line,
                    });
                    while w.lines.len() > LINE_BUFFER_CAP {
                        w.lines.remove(0);
                    }
                } else {
                    dioxus_utils::console_log(&format!("[logs-ws] non-line payload: {text}"));
                }
            }
            Ok(Message::Bytes(_)) => {}
            Err(err) => {
                dioxus_utils::console_log(&format!("[logs-ws] error: {err:?}"));
                let mut w = cs.write();
                w.live = false;
                push_marker(&mut w.lines, &format!("── disconnected ({err:?}) ──"));
                return;
            }
        }
    }
    dioxus_utils::console_log(&format!("[logs-ws] stream ended after {received} msgs"));
    let mut w = cs.write();
    w.live = false;
    push_marker(&mut w.lines, "── disconnected ──");
}

fn build_ws_url(env: &str, container_id: &str) -> String {
    let origin = get_base_url();
    let scheme = if origin.starts_with("https://") {
        "wss"
    } else {
        "ws"
    };
    let host = origin
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    format!("{scheme}://{host}/ws/logs?env={env}&id={container_id}")
}

#[derive(Default)]
struct LogPreviewState {
    lines: Vec<LogLineHttpModel>,
    /// id of the container the buffer currently belongs to; used to decide
    /// whether to keep or clear the buffer when the resource future restarts.
    current_id: Option<String>,
    /// true while the WS task is alive and consuming messages.
    live: bool,
    /// false until the first batch of lines has been auto-scrolled to bottom;
    /// re-armed when the user switches container.
    did_initial_scroll: bool,
    error: Option<String>,
}
