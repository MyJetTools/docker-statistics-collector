use dioxus::prelude::*;
use dioxus_utils::*;

use crate::api::{get_logs, LogLineHttpModel};

#[component]
pub fn show_logs(env: String, url: String, container_id: String) -> Element {
    let mut dialog_state = use_signal(|| DialogState::new());
    let dialog_state_read_access = dialog_state.read();

    let items = match dialog_state_read_access.data.as_ref() {
        RenderState::None => {
            let lines_amount_value = dialog_state_read_access.get_lines_amount();
            spawn(async move {
                dialog_state.write().data.set_loading();
                let result = get_logs(env, url, container_id, lines_amount_value).await;

                match result {
                    Ok(result) => {
                        dialog_state.write().data.set_loaded(result);
                    }
                    Err(err) => {
                        dialog_state.write().data.set_error(err.to_string());
                    }
                }
            });

            return rsx! {
                {"Loading logs..."}
            };
        }
        RenderState::Loading => {
            return rsx! {
                {"Loading logs..."}
            };
        }
        RenderState::Loaded(items) => items,
        RenderState::Error(err) => {
            let msg = format!("Error during receiving logs: {:?}", err);
            return rsx! {
                div { style: "color:red", {msg} }
            };
        }
    };

    //    let mut lines_amount = use_signal(|| 100u32);
    //    let lines_amount_value = *lines_amount.read();

    let amount_value = dialog_state_read_access.lines_amount.to_string();

    rsx! {
        div { class: "modal-content",
            div { class: "input-group",
                span { class: "input-group-text", "Amount" }
                input {
                    class: "form-control",
                    value: "{amount_value}",
                    r#type: "number",
                    onchange: move |cx| {
                        dialog_state.write().lines_amount = cx.value();
                    },
                }
                button {
                    class: "btn btn-outline-secondary",
                    onclick: move |_| {
                        dialog_state.write().data.reset();
                    },
                    "Request"
                }
            }

            div {
                style: "height:80vh; font-size: 14px; margin-top:10px",
                class: "form-control modal-content-full-screen",
                {render_logs_content(items)}
            }
        }
    }
}

fn render_logs_content(content: &[LogLineHttpModel]) -> Element {
    let mut items_to_render = Vec::new();

    for line in content {
        let cl = match line.tp {
            0 => "orange",
            1 => "black",
            2 => "red",
            _ => "gray",
        };
        items_to_render.push(rsx! {
            div { style: "color: {cl}", "{line.line.as_str()}" }
        });
    }

    rsx! {
        {items_to_render.into_iter()}
    }
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

