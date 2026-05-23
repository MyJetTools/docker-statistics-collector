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
/// Initial backfill (tail=N) requested from the collector on every reconnect.
const INITIAL_TAIL: u32 = 200;

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

    // Bump session_id whenever container_id changes — the live WS task watches
    // this and exits as soon as a new session starts, so we never have two
    // streams writing into the same buffer.
    let cid_for_effect = container_id.clone();
    let env_for_effect = env.clone();
    use_effect(use_reactive!(|cid_for_effect| {
        let mut cs = cs.to_owned();
        {
            let mut w = cs.write();
            w.session_id += 1;
            w.lines.clear();
            w.error = None;
        }
        let my_session = cs.read().session_id;
        spawn_stream(cs, env_for_effect.clone(), cid_for_effect.clone(), my_session);
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

fn spawn_stream(
    mut cs: Signal<LogPreviewState>,
    env: Rc<String>,
    container_id: String,
    my_session: u64,
) {
    spawn(async move {
        let ws_url = build_ws_url(env.as_str(), &container_id);
        dioxus_utils::console_log(&format!(
            "[logs-ws] opening session={my_session} url={ws_url}"
        ));
        let ws = match WebSocket::open(&ws_url) {
            Ok(ws) => ws,
            Err(err) => {
                dioxus_utils::console_log(&format!(
                    "[logs-ws] open failed session={my_session}: {err:?}"
                ));
                if cs.read().session_id == my_session {
                    cs.write().error = Some(format!("open WS failed: {err:?}"));
                }
                return;
            }
        };

        let (_write, mut read) = ws.split();
        dioxus_utils::console_log(&format!(
            "[logs-ws] connected session={my_session}, waiting for first message…"
        ));

        let mut received = 0u64;
        while let Some(msg) = read.next().await {
            // Another session started — drop everything.
            if cs.read().session_id != my_session {
                dioxus_utils::console_log(&format!(
                    "[logs-ws] session={my_session} superseded after {received} msgs — exiting"
                ));
                return;
            }
            match msg {
                Ok(Message::Text(text)) => {
                    received += 1;
                    if received <= 3 {
                        let preview: String = text.chars().take(200).collect();
                        dioxus_utils::console_log(&format!(
                            "[logs-ws] session={my_session} msg #{received}: {preview}"
                        ));
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
                        dioxus_utils::console_log(&format!(
                            "[logs-ws] session={my_session} non-line payload: {text}"
                        ));
                    }
                }
                Ok(Message::Bytes(_)) => {}
                Err(err) => {
                    dioxus_utils::console_log(&format!(
                        "[logs-ws] session={my_session} error: {err:?}"
                    ));
                    if cs.read().session_id == my_session {
                        cs.write().error = Some(format!("WS error: {err:?}"));
                    }
                    return;
                }
            }
        }
        dioxus_utils::console_log(&format!(
            "[logs-ws] session={my_session} stream ended after {received} msgs"
        ));
    });
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
    session_id: u64,
    error: Option<String>,
}
