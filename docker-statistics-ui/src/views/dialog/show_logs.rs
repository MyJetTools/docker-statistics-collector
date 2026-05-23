use dioxus::prelude::*;
use dioxus_utils::*;

use crate::api::{get_logs, LogLineHttpModel};

#[component]
pub fn show_logs(env: String, url: String, container_id: String) -> Element {
    let mut dialog_state = use_signal(|| DialogState::new());
    let dialog_state_ra = dialog_state.read();

    let items = match dialog_state_ra.data.as_ref() {
        RenderState::None => {
            let lines_amount_value = dialog_state_ra.get_lines_amount();
            spawn(async move {
                dialog_state.write().data.set_loading();
                let result = get_logs(env, url, container_id, lines_amount_value).await;
                match result {
                    Ok(result) => dialog_state.write().data.set_loaded(result),
                    Err(err) => dialog_state.write().data.set_error(err.to_string()),
                }
            });

            return rsx! { div { class: "ds-loading", "Loading logs…" } };
        }
        RenderState::Loading => return rsx! { div { class: "ds-loading", "Loading logs…" } },
        RenderState::Loaded(items) => items,
        RenderState::Error(err) => {
            let msg = format!("Error fetching logs: {:?}", err);
            return rsx! { div { class: "ds-error", "{msg}" } };
        }
    };

    let amount_value = dialog_state_ra.lines_amount.clone();

    rsx! {
        div { class: "ds-modal-toolbar",
            div { class: "ds-input-group",
                span { class: "label", "lines" }
                input {
                    r#type: "number",
                    value: "{amount_value}",
                    onchange: move |evt| {
                        dialog_state.write().lines_amount = evt.value();
                    },
                }
                button {
                    class: "btn",
                    onclick: move |_| { dialog_state.write().data.reset(); },
                    "fetch"
                }
            }
            span { style: "color:var(--text-muted); font-family:var(--mono); font-size:11px;",
                "{items.len()} lines"
            }
        }
        div { class: "ds-modal-scroll",
            {render_logs_content(items)}
        }
    }
}

fn render_logs_content(content: &[LogLineHttpModel]) -> Element {
    rsx! {
        for line in content.iter() {
            LogRow { tp: line.tp as i32, line: line.line.clone() }
        }
    }
}

#[component]
fn LogRow(tp: i32, line: String) -> Element {
    let lvl_class = match tp {
        0 => "log-line lvl-warn",
        1 => "log-line lvl-info",
        2 => "log-line lvl-err",
        _ => "log-line",
    };
    rsx! { div { class: "{lvl_class}", "{line}" } }
}

pub struct DialogState {
    data: DataState<Vec<LogLineHttpModel>>,
    lines_amount: String,
}

impl DialogState {
    pub fn new() -> Self {
        Self {
            data: Default::default(),
            lines_amount: "100".to_string(),
        }
    }

    pub fn get_lines_amount(&self) -> u32 {
        self.lines_amount.parse().unwrap_or(100)
    }
}
