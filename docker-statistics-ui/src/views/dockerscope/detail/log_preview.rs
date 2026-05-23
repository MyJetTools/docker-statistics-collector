use std::rc::Rc;

use dioxus::prelude::*;

use crate::states::{DialogState, DialogType, MainState};

/// Placeholder log preview — clicking "open" launches the full logs dialog (existing API).
/// Live tail will land in a later slice when the backend exposes a stream endpoint.
#[component]
pub fn LogPreview(container_id: String, vm_url: String, is_running: bool) -> Element {
    let status = if is_running { "● live" } else { "○ paused" };
    let status_color = if is_running {
        "var(--accent)"
    } else {
        "var(--text-muted)"
    };

    let env = consume_context::<Signal<MainState>>()
        .read()
        .envs
        .get_selected_env()
        .map(|e| e.clone())
        .unwrap_or_else(|| Rc::new(String::new()));

    let dialog_handle = consume_context::<Signal<DialogState>>();
    let id_for_click = container_id.clone();
    let url_for_click = vm_url.clone();

    rsx! {
        div { class: "panel",
            div { class: "panel-head",
                h3 { "Log tail · stdout" }
                span { class: "count-pill", style: "color: {status_color};", "{status}" }
            }
            div { class: "log-mini", style: "max-height: 160px;",
                div { style: "color: var(--text-muted); padding: 8px 0;",
                    "open full log viewer to stream output →"
                }
            }
            div { style: "margin-top:10px; display:flex; gap:8px;",
                button {
                    class: "btn",
                    onclick: move |_| {
                        dialog_handle.clone().write().show_dialog(
                            "Logs".to_string(),
                            DialogType::ShowLogs {
                                env: env.clone(),
                                url: url_for_click.clone(),
                                container_id: id_for_click.clone(),
                            },
                        );
                    },
                    "open logs"
                }
            }
        }
    }
}
