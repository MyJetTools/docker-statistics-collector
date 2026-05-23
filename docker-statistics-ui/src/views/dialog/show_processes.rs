use std::time::Duration;

use dioxus::prelude::*;
use dioxus_utils::*;

use crate::api::{get_processes, ProcessHttpModel};
use crate::states::{DialogState, DialogType};
use crate::utils::format_mem;

#[component]
pub fn show_processes(env: String, url: String, container_id: String) -> Element {
    let mut state = use_signal(|| ProcessesDialogState::new());

    // Background poller: refreshes every 1s without going through Loading, so the table doesn't flash.
    // Self-terminates when DialogState no longer shows ShowProcesses for THIS container_id
    // (dialog closed, or user opened the dialog for a different container).
    let dialog_signal = consume_context::<Signal<DialogState>>();
    {
        let env_l = env.clone();
        let url_l = url.clone();
        let id_l = container_id.clone();
        use_effect(move || {
            let env_l = env_l.clone();
            let url_l = url_l.clone();
            let id_l = id_l.clone();
            spawn(async move {
                loop {
                    dioxus_utils::js::sleep(Duration::from_secs(1)).await;
                    if !is_still_active(dialog_signal, &id_l) {
                        break;
                    }
                    let result = get_processes(env_l.clone(), url_l.clone(), id_l.clone()).await;
                    match result {
                        Ok(items) => state.write().data.set_loaded(items),
                        Err(err) => state.write().data.set_error(err.to_string()),
                    }
                }
            });
        });
    }

    let state_ra = state.read();

    let items = match state_ra.data.as_ref() {
        RenderState::None => {
            spawn(async move {
                state.write().data.set_loading();
                let result = get_processes(env, url, container_id).await;
                match result {
                    Ok(result) => state.write().data.set_loaded(result),
                    Err(err) => state.write().data.set_error(err.to_string()),
                }
            });
            return rsx! { div { class: "ds-loading", "Loading processes…" } };
        }
        RenderState::Loading => return rsx! { div { class: "ds-loading", "Loading processes…" } },
        RenderState::Loaded(items) => items,
        RenderState::Error(err) => {
            let msg = format!("Error loading processes: {:?}", err);
            return rsx! { div { class: "ds-error", "{msg}" } };
        }
    };

    rsx! {
        div { class: "ds-modal-toolbar",
            span { style: "color:var(--text-muted); font-family:var(--mono); font-size:11px;",
                "{items.len()} processes · auto-refresh 1s"
            }
        }
        div { class: "ds-modal-scroll",
            {render_processes_table(items)}
        }
    }
}

fn is_still_active(dialog: Signal<DialogState>, my_id: &str) -> bool {
    let ra = dialog.read();
    match &*ra {
        DialogState::Shown {
            dialog_type: DialogType::ShowProcesses { container_id, .. },
            ..
        } => container_id == my_id,
        _ => false,
    }
}

fn render_processes_table(processes: &[ProcessHttpModel]) -> Element {
    // Busiest process first — the one to worry about.
    let mut sorted: Vec<&ProcessHttpModel> = processes.iter().collect();
    sorted.sort_by(|a, b| b.open_files.unwrap_or(-1).cmp(&a.open_files.unwrap_or(-1)));

    rsx! {
        table {
            thead {
                tr {
                    th { "PID" }
                    th { "Open" }
                    th { "Limit" }
                    th { "RSS" }
                    th { "Virt" }
                    th { "Threads" }
                    th { "Command" }
                }
            }
            tbody {
                for p in sorted.into_iter() {
                    ProcessRow { p: p.clone() }
                }
            }
        }
    }
}

#[component]
fn ProcessRow(p: ProcessHttpModel) -> Element {
    let open = p
        .open_files
        .map(|v| v.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let limit = p
        .fd_limit
        .map(|v| v.to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let color = match (p.open_files, p.fd_limit) {
        (Some(o), Some(l)) if l > 0 => {
            let ratio = o as f64 / l as f64;
            if ratio >= 0.9 {
                "color: var(--danger);"
            } else if ratio >= 0.7 {
                "color: var(--warn);"
            } else {
                "color: var(--accent);"
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
            td { class: "mono", "{p.pid}" }
            td { class: "mono", style: "font-weight:600; {color}", "{open}" }
            td { class: "mono", "{limit}" }
            td { class: "mono", "{rss}" }
            td { class: "mono", style: "color: var(--text-muted);", "{virt}" }
            td { class: "mono", "{threads}" }
            td {
                div {
                    style: "font-family: var(--mono); font-size:11.5px; max-width:600px; overflow-x:auto; white-space:nowrap; color:var(--text);",
                    "{p.cmd}"
                }
            }
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
