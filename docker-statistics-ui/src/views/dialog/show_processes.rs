use dioxus::prelude::*;
use dioxus_utils::*;

use crate::api::{get_processes, ProcessHttpModel};
use crate::utils::format_mem;

#[component]
pub fn show_processes(env: String, url: String, container_id: String) -> Element {
    let mut state = use_signal(|| ProcessesDialogState::new());
    let state_read_access = state.read();

    let items = match state_read_access.data.as_ref() {
        RenderState::None => {
            spawn(async move {
                state.write().data.set_loading();
                let result = get_processes(env, url, container_id).await;

                match result {
                    Ok(result) => {
                        state.write().data.set_loaded(result);
                    }
                    Err(err) => {
                        state.write().data.set_error(err.to_string());
                    }
                }
            });

            return rsx! {
                {"Loading processes..."}
            };
        }
        RenderState::Loading => {
            return rsx! {
                {"Loading processes..."}
            };
        }
        RenderState::Loaded(items) => items,
        RenderState::Error(err) => {
            let msg = format!("Error loading processes: {:?}", err);
            return rsx! {
                div { style: "color:red", {msg} }
            };
        }
    };

    rsx! {
        div { class: "modal-content",
            div { class: "input-group",
                span { class: "input-group-text", "Processes: {items.len()}" }
                button {
                    class: "btn btn-outline-secondary",
                    onclick: move |_| {
                        state.write().data.reset();
                    },
                    "Refresh"
                }
            }

            div {
                style: "height:80vh; font-size: 14px; margin-top:10px; overflow:auto",
                class: "form-control modal-content-full-screen",
                {render_processes_table(items)}
            }
        }
    }
}

fn render_processes_table(processes: &[ProcessHttpModel]) -> Element {
    // Busiest process first — the one to worry about.
    let mut sorted: Vec<&ProcessHttpModel> = processes.iter().collect();
    sorted.sort_by(|a, b| b.open_files.unwrap_or(-1).cmp(&a.open_files.unwrap_or(-1)));

    let rows = sorted.into_iter().map(|p| {
        let open = p
            .open_files
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let limit = p
            .fd_limit
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string());

        let color = match (p.open_files, p.fd_limit) {
            (Some(open), Some(limit)) if limit > 0 => {
                let ratio = open as f64 / limit as f64;
                if ratio >= 0.9 {
                    "color:red"
                } else if ratio >= 0.7 {
                    "color:darkorange"
                } else {
                    "color:green"
                }
            }
            _ => "",
        };

        let rss = p
            .mem_rss
            .map(format_mem)
            .unwrap_or_else(|| "N/A".to_string());
        let virt = p
            .mem_vsize
            .map(format_mem)
            .unwrap_or_else(|| "N/A".to_string());
        let threads = p
            .threads
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string());

        rsx! {
            tr {
                td { style: "padding-right: 20px", "{p.pid}" }
                td { style: "{color}; font-weight:bold; padding-right: 20px", "{open}" }
                td { style: "padding-right: 20px", "{limit}" }
                td { style: "padding-right: 20px", "{rss}" }
                td { style: "padding-right: 20px; color:gray", "{virt}" }
                td { style: "padding-right: 20px", "{threads}" }
                td {
                    div {
                        style: "font-family:monospace; font-size:12px; max-width: 800px; overflow-x: auto; white-space: nowrap",
                        "{p.cmd}"
                    }
                }
            }
        }
    });

    rsx! {
        table { class: "table table-sm", style: "text-align:left",
            tr {
                th { style: "padding-right: 20px", "PID" }
                th { style: "padding-right: 20px", "Open files" }
                th { style: "padding-right: 20px", "Limit" }
                th { style: "padding-right: 20px", title: "Resident memory (VmRSS) — what's actually in RAM", "RSS" }
                th { style: "padding-right: 20px; color:gray", title: "Virtual memory (VmSize) — full allocated address space", "Virt" }
                th { style: "padding-right: 20px", "Threads" }
                th { "Command" }
            }
            {rows}
        }
    }
}

pub struct ProcessesDialogState {
    data: DataState<Vec<ProcessHttpModel>>,
}

impl ProcessesDialogState {
    pub fn new() -> Self {
        Self {
            data: Default::default(),
        }
    }
}
