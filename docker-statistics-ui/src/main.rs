#![allow(non_snake_case)]

use std::time::Duration;

mod api;

use api::*;
use dioxus::prelude::*;
use dioxus_utils::*;

mod models;

mod router;
mod selected_vm;
mod utils;

mod states;

mod views;

pub use router::*;
use views::*;

use crate::states::*;

fn main() {
    dioxus::LaunchBuilder::new().launch(|| {
        rsx! {
            document::Link { rel: "icon", href: "/assets/favicon.ico" }
            document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
            document::Link {
                rel: "preconnect",
                href: "https://fonts.gstatic.com",
                crossorigin: "anonymous",
            }
            document::Link {
                rel: "stylesheet",
                href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap",
            }
            Router::<AppRoute> {}
        }
    })
}

/// Layout shared by every route — providers, env gating, the 3-column UI, plus
/// an `Outlet` so the per-route leaf components can sync URL ↔ state.
#[component]
pub fn AppShell() -> Element {
    use_context_provider(|| Signal::new(MainState::new()));
    use_context_provider(|| Signal::new(DialogState::Hidden));
    use_context_provider(|| Signal::new(Prefs::load()));

    // Apply persisted theme to <html> so CSS variable cascade reaches body + scrollbars + chart vars.
    let prefs = consume_context::<Signal<Prefs>>();
    use_effect(move || {
        let theme = prefs.read().theme.data_attr();
        let _ = dioxus_utils::eval(&format!(
            "document.documentElement.setAttribute('data-theme','{}')",
            theme
        ));
    });

    let mut main_state = consume_context::<Signal<MainState>>();
    let main_state_ra = main_state.read();

    match main_state_ra.envs.items.as_ref() {
        RenderState::None => {
            spawn(async move {
                let envs = get_envs().await;
                match envs {
                    Ok(envs) => {
                        let mut w = main_state.write();
                        w.envs.set_items(envs.envs);
                        w.prompt_pass_key = envs.request_pass_key;
                    }
                    Err(err) => {
                        main_state.write().envs.set_error(err.to_string());
                    }
                }
            });
            return rsx! { div { class: "ds-loading", "Loading environments…" } };
        }
        RenderState::Loading => {
            return rsx! { div { class: "ds-loading", "Loading environments…" } };
        }
        RenderState::Loaded(_) => {}
        RenderState::Error(err) => {
            let msg = format!("Error loading environments. Err: {}", err);
            return rsx! { div { class: "ds-error", "{msg}" } };
        }
    }

    if main_state_ra.prompt_pass_key {
        return rsx! { PromptSshPassKey {} };
    }

    let env = main_state_ra.envs.get_selected_env();
    drop(main_state_ra);
    let Some(env) = env else {
        return rsx! { div { class: "ds-loading", "no env selected" } };
    };

    let mut started = use_signal(|| false);
    use_effect(move || {
        started.set(true);
        read_loop(main_state);
    });

    rsx! {
        div { class: "app density-cozy",
            Topbar {}
            VmRail {}
            ContainerListPanel {}
            DetailPanel { env: env.clone() }
            dialog::render_dialog {}
        }
        Outlet::<AppRoute> {}
    }
}

pub fn read_loop(mut main_state: Signal<MainState>) {
    spawn(async move {
        loop {
            dioxus_utils::js::sleep(Duration::from_secs(1)).await;
            let (env, selected_vm) = { main_state.read().get_selected_vm() };

            let selected_vm = match selected_vm {
                Some(value) => value.to_string(),
                None => "".to_string(),
            };

            let result = get_vm_cpu_and_mem(env, selected_vm).await;

            match result {
                Ok(result) => {
                    let mut write_state = main_state.write();
                    write_state.vms_state = result.vms;
                    if let Some(metrics) = result.metrics {
                        write_state.set_containers(metrics);
                    } else {
                        write_state.set_containers(Vec::new());
                    }
                }
                Err(err) => {
                    dioxus_utils::console_log(&format!(
                        "Error on get_vm_cpu_and_mem: {:?}",
                        err
                    ));
                }
            }
        }
    });
}
