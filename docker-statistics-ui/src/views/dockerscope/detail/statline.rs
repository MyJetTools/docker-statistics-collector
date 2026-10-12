use dioxus::prelude::*;

use crate::models::ContainerModel;
use crate::states::{DialogState, DialogType, ServiceHistoryModel};
use crate::views::dockerscope::helpers::pct;

/// The key figures of a container, under its graphs. The open files have no graph
/// of their own up there, so their figure is the way into their history —
/// `files_history` is what the dialog is opened with.
#[component]
pub fn StatLine(container: ContainerModel, files_history: ServiceHistoryModel) -> Element {
    let cpu = container.cpu.usage.unwrap_or(0.0);
    let mem_used = container.mem.usage.unwrap_or(0);
    let mem_limit = container.mem.limit.unwrap_or(0);
    let mem_pct = pct(mem_used, mem_limit) as i32;
    let files_open = container.files.open.unwrap_or(0);
    let files_limit = container.files.limit.unwrap_or(0);
    let files_pct = pct(files_open, files_limit) as i32;

    rsx! {
        div { class: "statline",
            Stat { k: "CPU usage", v: format!("{:.2}", cpu), u: "%".to_string() }
            Stat { k: "Memory %",   v: format!("{}",     mem_pct), u: "%".to_string() }
            Stat {
                k: "Open FDs",
                v: format!("{}", files_open),
                u: format!("/ {}", files_limit),
                history: files_history,
            }
            Stat { k: "FD %",       v: format!("{}",     files_pct), u: "%".to_string() }
        }
    }
}

#[component]
fn Stat(k: String, v: String, u: String, history: Option<ServiceHistoryModel>) -> Element {
    rsx! {
        div { class: "stat",
            match history {
                // Pressed like the name of a graph: it opens the history of the figure
                Some(history) => rsx! {
                    div {
                        class: "k history",
                        title: "the whole history of {k}",
                        onclick: move |_| {
                            consume_context::<Signal<DialogState>>().write().show_dialog(
                                history.title(),
                                DialogType::ServiceHistory(history.clone()),
                            );
                        },
                        "{k}"
                    }
                },
                None => rsx! {
                    div { class: "k", "{k}" }
                },
            }
            div { class: "v",
                "{v}"
                span { class: "u", "{u}" }
            }
        }
    }
}
