use dioxus::prelude::*;

use crate::states::{DialogState, EditDiskTitleModel};
use crate::views::dialog::EditDiskTitleState;

/// Longest title the api accepts (`MAX_DISK_TITLE_LEN` on its side). The box
/// stops there, so the limit is met while typing rather than on save.
const MAX_TITLE_LEN: i64 = 48;

/// One box: the name a host disk goes by in place of its mount point. An empty
/// box removes the name. The dialog only collects the text — saving it is the
/// router's job, reached through `on_submit`.
#[component]
pub fn edit_disk_title(model: EditDiskTitleModel, on_submit: EventHandler<String>) -> Element {
    let mut cs = use_signal(|| EditDiskTitleState::new(&model));
    let cs_ra = cs.read();
    let value = cs_ra.value.as_str();

    let save_label = if model.saving { "saving…" } else { "save" };

    rsx! {
        div { class: "disk-title-form",
            div { class: "disk-title-target",
                "{model.vm} · {model.mount_point} · {model.device}"
            }
            input {
                class: "ds-input",
                value: "{value}",
                placeholder: "{model.mount_point}",
                maxlength: MAX_TITLE_LEN,
                disabled: model.saving,
                // The dialog exists to type into this box, so it opens with the
                // caret already in it.
                onmounted: move |evt| async move {
                    let _ = evt.set_focus(true).await;
                },
                oninput: move |evt| cs.write().set_value(evt.value()),
                onkeydown: move |evt| match evt.key() {
                    Key::Enter => on_submit.call(cs.read().value.clone()),
                    Key::Escape => consume_context::<Signal<DialogState>>().write().hide_dialog(),
                    _ => {}
                },
            }
            div { class: "disk-title-hint",
                "Shown in place of the mount point. Leave it empty to go back to "
                code { "{model.mount_point}" }
                "."
            }
            if let Some(error) = model.error.as_ref() {
                div { class: "ds-error", "{error}" }
            }
            div { class: "disk-title-actions",
                button {
                    class: "btn",
                    onclick: move |_| {
                        consume_context::<Signal<DialogState>>().write().hide_dialog();
                    },
                    "cancel"
                }
                button {
                    class: "btn primary",
                    disabled: model.saving,
                    onclick: move |_| on_submit.call(cs.read().value.clone()),
                    "{save_label}"
                }
            }
        }
    }
}
