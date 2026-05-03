use dioxus::prelude::*;
use dioxus_utils::*;
use serde::{Deserialize, Serialize};

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

/*
fn load_logs<'s>(
    cx: &'s Scope<'s, ShowPopulatedYamlProps>,
    state: &UseState<String>,
    url: String,
    container_id: String,
    lines_amount: u32,
) {
    let state = state.to_owned();

    cx.spawn(async move {
        let mut lines = tokio::spawn(async move {
            let result = get_logs(url, container_id, lines_amount).await;

            match result {
                Ok(result) => result,
                Err(err) => format!("Error during receiving logs: {:?}", err),
            }
        })
        .await
        .unwrap();

        if lines.len() == 0 {
            lines = "No Logs Received".to_string();
        }

        state.set(lines);
    });
}
 */

#[derive(Debug, Serialize, Deserialize)]
pub struct LogLineHttpModel {
    pub tp: u8,
    pub line: String,
}

#[server]
async fn get_logs(
    env: String,
    url: String,
    id: String,
    lines_amount: u32,
) -> Result<Vec<LogLineHttpModel>, ServerFnError> {
    let fl_url = crate::server::APP_CTX
        .get_fl_url(env.as_str(), url.as_str())
        .await;
    let result = crate::server::http_client::get_logs(fl_url, id, lines_amount).await;
    let payload = match result {
        Ok(result) => result,
        Err(err) => return Err(ServerFnError::new(err)),
    };

    if payload.len() == 0 {
        return Ok(vec![]);
    }
    let mut result = Vec::new();

    println!("Payload.len = {}", payload.len());

    let mut payload = payload.into_iter();
    loop {
        let tp = payload.next();

        if tp.is_none() {
            break;
        }

        let tp = tp.unwrap();

        let n = payload.next().unwrap_or(255);
        if n != 0 {
            break;
        }
        let n = payload.next().unwrap_or(255);
        if n != 0 {
            break;
        }

        payload.next().unwrap_or(255);
        if n != 0 {
            break;
        }

        let mut size = [0u8; 4];

        size[0] = payload.next().unwrap();
        size[1] = payload.next().unwrap();
        size[2] = payload.next().unwrap();
        size[3] = payload.next().unwrap();

        let size = u32::from_be_bytes(size) as usize;

        let mut str = Vec::with_capacity(size);

        for _ in 0..size - 1 {
            str.push(payload.next().unwrap());
        }

        payload.next().unwrap();

        let item = LogLineHttpModel {
            tp,
            line: String::from_utf8(str).unwrap(),
        };

        result.push(item);
    }

    Ok(result)
}
