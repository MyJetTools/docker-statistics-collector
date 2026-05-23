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

    // Buffer of received lines + any terminal error. Owned by a signal so the
    // streaming task can keep pushing without us re-rendering anything else.
    let cs = use_signal(LogPreviewState::default);

    // `use_resource` keeps a single async task alive for as long as deps stay
    // the same. When container_id (or env) actually changes, the previous
    // future is dropped — which drops our `WebSocket` and closes the upstream
    // cleanly — and a fresh task starts. Re-renders that don't change deps do
    // NOT restart the task.
    let env_for_res = env.clone();
    let container_for_res = container_id.clone();
    let _stream_task = use_resource(use_reactive!(|container_for_res| {
        let env = env_for_res.clone();
        let mut cs = cs.to_owned();
        async move {
            // Clear stale buffer when we swap container.
            {
                let mut w = cs.write();
                w.lines.clear();
                w.error = None;
            }
            run_stream(cs, env, container_for_res).await;
        }
    }));

    let status = if is_running { "● live" } else { "○ paused" };
    let status_color = if is_running {
        "var(--accent)"
    } else {
        "var(--text-muted)"
    };

    let body = render_body(&cs.read());

    let cid_for_modal = container_id.clone();
    let url_for_modal = vm_url.clone();
    let env_for_modal = env.clone();
    let dialog_handle = consume_context::<Signal<DialogState>>();

    rsx! {
        div { class: "panel log-panel",
            div { class: "panel-head",
                h3 { "Log tail · live · last {LINE_BUFFER_CAP} lines" }
                div { style: "display:flex; gap:8px; align-items:center;",
                    span { class: "count-pill", style: "color: {status_color};", "{status}" }
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
                }
            }
            div { class: "log-mini log-mini-tall", {body} }
        }
    }
}

fn render_body(cs: &LogPreviewState) -> Element {
    if let Some(err) = cs.error.as_deref() {
        return rsx! { div { class: "ds-error", "log stream error: {err}" } };
    }
    if cs.lines.is_empty() {
        return rsx! {
            div { style: "padding:6px 0; color:var(--text-muted);", "waiting for log lines…" }
        };
    }
    rsx! {
        for line in cs.lines.iter() {
            LogLine { tp: line.tp as i32, text: line.line.clone() }
        }
    }
}

#[component]
fn LogLine(tp: i32, text: String) -> Element {
    let cls = match tp {
        0 => "log-line lvl-warn",
        1 => "log-line lvl-info",
        2 => "log-line lvl-err",
        _ => "log-line",
    };
    rsx! { div { class: "{cls}", "{text}" } }
}

async fn run_stream(mut cs: Signal<LogPreviewState>, env: Rc<String>, container_id: String) {
    let ws_url = build_ws_url(env.as_str(), &container_id);
    dioxus_utils::console_log(&format!("[logs-ws] opening url={ws_url}"));

    let ws = match WebSocket::open(&ws_url) {
        Ok(ws) => ws,
        Err(err) => {
            dioxus_utils::console_log(&format!("[logs-ws] open failed: {err:?}"));
            cs.write().error = Some(format!("open WS failed: {err:?}"));
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
                if received <= 3 {
                    let preview: String = text.chars().take(200).collect();
                    dioxus_utils::console_log(&format!("[logs-ws] msg #{received}: {preview}"));
                }
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
                cs.write().error = Some(format!("WS error: {err:?}"));
                return;
            }
        }
    }
    dioxus_utils::console_log(&format!("[logs-ws] stream ended after {received} msgs"));
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
    error: Option<String>,
}
