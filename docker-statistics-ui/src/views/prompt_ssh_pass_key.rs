use dioxus::prelude::*;

use crate::api::apply_pass_phrase;
use crate::MainState;

#[component]
pub fn PromptSshPassKey() -> Element {
    let mut cs = use_signal(|| PromptSshPassKeyState::new());
    let cs_ra = cs.read();
    let pass = cs_ra.pass_phrase.clone();

    rsx! {
        div { id: "dialog-pad",
            div { class: "ds-modal narrow",
                div { class: "ds-modal-head", h5 { "Enter SSH Pass Key" } }
                div { class: "ds-modal-body",
                    p { style: "color: var(--text-dim); font-family: var(--mono); font-size: 12px; margin: 0 0 12px;",
                        "Enter the SSH pass key to use for the SSH connection."
                    }
                    input {
                        class: "ds-input",
                        r#type: "password",
                        placeholder: "SSH pass key",
                        value: "{pass}",
                        oninput: move |evt| {
                            cs.write().pass_phrase = evt.value();
                        },
                    }
                    div { style: "margin-top: 14px; display: flex; justify-content: flex-end;",
                        button {
                            class: "btn",
                            style: "background: var(--accent); color: #04130a; border-color: var(--accent); font-weight: 600;",
                            onclick: move |_| {
                                let pass_phrase = cs.read().pass_phrase.clone();
                                spawn(async move {
                                    apply_pass_phrase(pass_phrase).await.unwrap();
                                    consume_context::<Signal<MainState>>().write().prompt_pass_key = false;
                                });
                            },
                            "Submit"
                        }
                    }
                }
            }
        }
    }
}

pub struct PromptSshPassKeyState {
    pub pass_phrase: String,
}

impl PromptSshPassKeyState {
    pub fn new() -> Self {
        Self {
            pass_phrase: String::new(),
        }
    }
}
