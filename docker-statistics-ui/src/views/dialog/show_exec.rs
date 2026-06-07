use std::rc::Rc;

use dioxus::prelude::*;
use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::{FutureExt, SinkExt, StreamExt};
use reqwasm::websocket::{futures::WebSocket, Message};
use serde::Deserialize;

use crate::api::get_base_url;

#[derive(Deserialize)]
struct ExecMsg {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    code: Option<i64>,
}

#[derive(Clone, PartialEq)]
struct ExecLine {
    kind: &'static str,
    text: String,
}

#[derive(Default)]
struct ExecState {
    lines: Vec<ExecLine>,
    connected: bool,
}

/// Interactive exec console dialog. Gated behind an explicit "allow" click; once
/// allowed it opens a WebSocket to `/ws/exec?env&id`, sends each typed command
/// as a text frame and renders the streamed output. Each command runs as
/// `sh -c` on the container — there is no persistent shell session.
#[component]
pub fn show_exec(env: Rc<String>, url: String, container_id: String) -> Element {
    let _ = url;
    let cs = use_signal(ExecState::default);
    let mut allowed = use_signal(|| false);
    let mut input = use_signal(String::new);
    let cmd_tx = use_signal(|| None::<UnboundedSender<String>>);

    let env_for_res = env.clone();
    let cid_for_res = container_id.clone();
    let allowed_dep = *allowed.read();
    let _task = use_resource(use_reactive!(|allowed_dep| {
        let env = env_for_res.clone();
        let cid = cid_for_res.clone();
        let cs = cs.to_owned();
        let cmd_tx = cmd_tx.to_owned();
        async move {
            if !allowed_dep {
                return;
            }
            run_console(cs, cmd_tx, env, cid).await;
        }
    }));

    // Snap to the bottom whenever a new line arrives.
    let line_count = cs.read().lines.len();
    use_effect(use_reactive!(|line_count| {
        if line_count > 0 {
            let _ = dioxus_utils::eval(
                "requestAnimationFrame(() => { \
                    const el = document.getElementById('exec-console-scroll'); \
                    if (el) el.scrollTop = el.scrollHeight; \
                });",
            );
        }
    }));

    if !allowed_dep {
        return rsx! {
            div { class: "ds-exec-gate",
                div { class: "ds-exec-warn",
                    "⚠ This opens a shell inside the container and runs arbitrary commands "
                    "(as root, depending on the image). Use with care."
                }
                button {
                    class: "btn btn-sm ds-exec-allow",
                    onclick: move |_| allowed.set(true),
                    "Разрешить выполнение / Allow exec"
                }
            }
        };
    }

    let cs_ra = cs.read();
    let connected = cs_ra.connected;
    let lines: Vec<ExecLine> = cs_ra.lines.clone();
    drop(cs_ra);

    let submit = move |_| {
        let command = input.read().trim().to_string();
        if command.is_empty() {
            return;
        }
        if let Some(tx) = cmd_tx.read().clone() {
            let _ = tx.unbounded_send(command);
        }
        input.set(String::new());
    };

    rsx! {
        div { class: "ds-exec-console",
            div { id: "exec-console-scroll", class: "ds-exec-output",
                if lines.is_empty() {
                    div { class: "ds-exec-line exec-info", "connecting…" }
                }
                for line in lines.iter() {
                    div { class: "ds-exec-line exec-{line.kind}", "{line.text}" }
                }
            }
            div { class: "ds-exec-inputrow",
                span { class: "ds-exec-prompt", "$" }
                input {
                    class: "ds-exec-input",
                    r#type: "text",
                    autofocus: true,
                    placeholder: if connected { "type a command and press Enter" } else { "connecting…" },
                    value: "{input}",
                    oninput: move |e| input.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            let command = input.read().trim().to_string();
                            if !command.is_empty() {
                                if let Some(tx) = cmd_tx.read().clone() {
                                    let _ = tx.unbounded_send(command);
                                }
                                input.set(String::new());
                            }
                        }
                    },
                }
                button { class: "btn btn-sm", onclick: submit, "Run" }
            }
        }
    }
}

async fn run_console(
    mut cs: Signal<ExecState>,
    mut cmd_tx: Signal<Option<UnboundedSender<String>>>,
    env: Rc<String>,
    container_id: String,
) {
    let ws_url = build_exec_ws_url(env.as_str(), &container_id);
    dioxus_utils::console_log(&format!("[exec-ws] opening url={ws_url}"));

    let ws = match WebSocket::open(&ws_url) {
        Ok(ws) => ws,
        Err(err) => {
            push(&mut cs, "error", format!("── open failed: {err:?} ──"));
            return;
        }
    };

    let (mut write, mut read) = ws.split();
    let (tx, mut rx) = unbounded::<String>();
    cmd_tx.set(Some(tx));
    cs.write().connected = true;

    loop {
        futures::select! {
            incoming = read.next().fuse() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(msg) = serde_json::from_str::<ExecMsg>(&text) {
                            append_msg(&mut cs, msg);
                        } else {
                            dioxus_utils::console_log(&format!("[exec-ws] non-json payload: {text}"));
                        }
                    }
                    Some(Ok(Message::Bytes(_))) => {}
                    Some(Err(err)) => {
                        push(&mut cs, "error", format!("── disconnected ({err:?}) ──"));
                        break;
                    }
                    None => {
                        push(&mut cs, "info", "── disconnected ──".to_string());
                        break;
                    }
                }
            }
            outgoing = rx.next().fuse() => {
                if let Some(command) = outgoing {
                    if write.send(Message::Text(command)).await.is_err() {
                        push(&mut cs, "error", "── send failed ──".to_string());
                        break;
                    }
                }
            }
        }
    }

    cs.write().connected = false;
}

fn append_msg(cs: &mut Signal<ExecState>, msg: ExecMsg) {
    match msg.kind.as_str() {
        "input" => push(cs, "input", format!("$ {}", msg.text.unwrap_or_default())),
        "output" => push(cs, "output", msg.text.unwrap_or_default()),
        "error" => push(cs, "error", msg.text.unwrap_or_default()),
        "info" => push(cs, "info", msg.text.unwrap_or_default()),
        "exit" => {
            let label = match msg.code {
                Some(c) => format!("[exit {c}]"),
                None => "[done]".to_string(),
            };
            push(cs, "exit", label);
        }
        _ => {}
    }
}

fn push(cs: &mut Signal<ExecState>, kind: &'static str, text: String) {
    cs.write().lines.push(ExecLine { kind, text });
}

fn build_exec_ws_url(env: &str, container_id: &str) -> String {
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
    format!("{scheme}://{host}/ws/exec?env={env}&id={container_id}")
}
