use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_utils::*;

use crate::api::{get_logs, LogLineHttpModel};
use crate::states::{DialogState, DialogType, MainState};

const TAIL_LINES: u32 = 200;

/// Inline log tail — fetches once on container change (no polling), renders
/// colored lines, plus a "reload" button and an "open full viewer" shortcut
/// to the existing modal.
#[component]
pub fn LogPreview(container_id: String, vm_url: String, is_running: bool) -> Element {
    let env = consume_context::<Signal<MainState>>()
        .read()
        .envs
        .get_selected_env()
        .map(|e| e.clone())
        .unwrap_or_else(|| Rc::new(String::new()));

    let cs = use_signal(LogPreviewState::default);

    // Re-fetch whenever container_id changes (new selection => fresh tail).
    let cid_for_effect = container_id.clone();
    let env_for_effect = env.clone();
    let url_for_effect = vm_url.clone();
    use_effect(use_reactive!(|cid_for_effect| {
        spawn_fetch(
            cs.to_owned(),
            env_for_effect.clone(),
            url_for_effect.clone(),
            cid_for_effect.clone(),
        );
    }));

    let status = if is_running { "● live" } else { "○ paused" };
    let status_color = if is_running {
        "var(--accent)"
    } else {
        "var(--text-muted)"
    };

    let body = render_body(&cs.read());

    let cid_for_reload = container_id.clone();
    let env_for_reload = env.clone();
    let url_for_reload = vm_url.clone();

    let cid_for_modal = container_id.clone();
    let url_for_modal = vm_url.clone();
    let env_for_modal = env.clone();
    let dialog_handle = consume_context::<Signal<DialogState>>();

    rsx! {
        div { class: "panel log-panel",
            div { class: "panel-head",
                h3 { "Log tail · stdout · last {TAIL_LINES} lines" }
                div { style: "display:flex; gap:8px; align-items:center;",
                    span { class: "count-pill", style: "color: {status_color};", "{status}" }
                    button {
                        class: "btn btn-sm",
                        onclick: move |_| {
                            spawn_fetch(
                                cs.to_owned(),
                                env_for_reload.clone(),
                                url_for_reload.clone(),
                                cid_for_reload.clone(),
                            );
                        },
                        "reload"
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
                }
            }
            div { class: "log-mini log-mini-tall", {body} }
        }
    }
}

fn render_body(cs: &LogPreviewState) -> Element {
    match cs.data.as_ref() {
        RenderState::None | RenderState::Loading => rsx! {
            div { style: "padding:6px 0; color:var(--text-muted);", "loading logs…" }
        },
        RenderState::Loaded(items) if items.is_empty() => rsx! {
            div { style: "padding:6px 0; color:var(--text-muted);", "no log output" }
        },
        RenderState::Loaded(items) => rsx! {
            for line in items.iter() {
                LogLine { tp: line.tp as i32, text: line.line.clone() }
            }
        },
        RenderState::Error(err) => {
            let msg = format!("error loading logs: {}", err);
            rsx! { div { class: "ds-error", "{msg}" } }
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

fn spawn_fetch(
    mut cs: Signal<LogPreviewState>,
    env: Rc<String>,
    url: String,
    container_id: String,
) {
    spawn(async move {
        cs.write().data.set_loading();
        let env_str = env.as_str().to_string();
        match get_logs(env_str, url, container_id, TAIL_LINES).await {
            Ok(items) => cs.write().data.set_loaded(items),
            Err(err) => cs.write().data.set_error(err.to_string()),
        }
    });
}

#[derive(Default)]
struct LogPreviewState {
    data: DataState<Vec<LogLineHttpModel>>,
}
