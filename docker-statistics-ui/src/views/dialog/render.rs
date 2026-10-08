use dioxus::prelude::*;

use crate::{
    states::{DialogState, DialogType, MainState},
    views::dialog::*,
};

pub fn render_dialog() -> Element {
    let dialog = consume_context::<Signal<DialogState>>();
    let main_state = consume_context::<Signal<MainState>>();
    let dialog_ra = dialog.read();

    match dialog_ra.as_ref() {
        DialogState::Hidden => rsx! {},
        DialogState::Shown { header, dialog_type } => {
            let content = match dialog_type {
                DialogType::ShowLogs { env, url, container_id } => rsx! {
                    show_logs {
                        env: env.clone(),
                        url: url.clone(),
                        container_id: container_id.clone(),
                    }
                },
                DialogType::ShowProcesses { env, url, container_id } => rsx! {
                    show_processes {
                        env: env.clone(),
                        url: url.clone(),
                        container_id: container_id.clone(),
                    }
                },
                DialogType::ShowExec { env, url, container_id } => rsx! {
                    show_exec {
                        env: env.clone(),
                        url: url.clone(),
                        container_id: container_id.clone(),
                    }
                },
                DialogType::EditDiskTitle(model) => {
                    let model_to_save = model.clone();
                    rsx! {
                        edit_disk_title {
                            model: model.clone(),
                            on_submit: move |title: String| {
                                save_disk_title(dialog, main_state, model_to_save.clone(), title);
                            },
                        }
                    }
                }
            };
            let header = header.clone();
            let modal_class = dialog_type.modal_class();

            rsx! {
                div { id: "dialog-pad",
                    div { class: "{modal_class}",
                        div { class: "ds-modal-head",
                            h5 { "{header}" }
                            button {
                                class: "btn btn-sm",
                                onclick: move |_| {
                                    consume_context::<Signal<DialogState>>().write().hide_dialog();
                                },
                                "✕"
                            }
                        }
                        div { class: "ds-modal-body", {content} }
                    }
                }
            }
        }
    }
}
