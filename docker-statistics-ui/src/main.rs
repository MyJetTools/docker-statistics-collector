#![allow(non_snake_case)]

use std::{collections::BTreeMap, env, time::Duration};

#[cfg(feature = "server")]
mod server;

use dioxus::prelude::*;
#[cfg(feature = "server")]
use dioxus::server::{IncrementalRendererConfig, ServeConfig};
use dioxus_utils::*;

mod models;

use models::*;

mod selected_vm;
mod utils;

mod states;

mod views;

use serde::*;
use views::*;

use crate::states::*;

pub const METRICS_HISTORY_SIZE: usize = 150;

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(server_only!(ServeConfig::builder().incremental(
            IncrementalRendererConfig::default()
                .invalidate_after(std::time::Duration::from_secs(120)),
        )))
        .launch(|| {
            rsx! {

                document::Link { rel: "icon", href: "/assets/favicon.ico" }
                App {}
            }
        })
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(MainState::new()));
    use_context_provider(|| Signal::new(DialogState::Hidden));

    let mut main_state = consume_context::<Signal<MainState>>();

    let main_state_read_access = main_state.read();

    match main_state_read_access.envs.items.as_ref() {
        RenderState::None => {
            spawn(async move {
                let envs = get_envs().await;
                match envs {
                    Ok(envs) => {
                        let mut main_state_write_access = main_state.write();
                        main_state_write_access.envs.set_items(envs.envs);
                        main_state_write_access.prompt_pass_key = envs.request_pass_key;
                    }
                    Err(err) => {
                        main_state.write().envs.set_error(err.to_string());
                    }
                }
            });
            return rsx! { "Loading environments..." };
        }
        RenderState::Loading => {
            return rsx! { "Loading environments..." };
        }
        RenderState::Loaded(_) => {}
        RenderState::Error(err) => {
            let err = format!("Error loading environments. Err: {}", err);
            return rsx! {
                {err}
            };
        }
    }

    if main_state_read_access.prompt_pass_key {
        return rsx! {
            PromptSshPassKey {}
        };
    }

    rsx! {
        ActiveApp {}
    }
}

#[component]
fn ActiveApp() -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let mut started = use_signal(|| false);

    let env = { main_state.read().envs.get_selected_env() };

    if env.is_none() {
        return rsx! { "No env selected" };
    }

    let env = env.unwrap();

    use_effect(move || {
        started.set(true);
        read_loop(main_state);
    });

    rsx! {

        div { id: "layout",
            div { id: "left-panel", left_panel {} }
            div { id: "right-panel",
                containers_list { env }
            }
            dialog::render_dialog {}
        }
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
                    write_state.vms_state = Some(result.vms);
                    if let Some(metrics) = result.metrics {
                        write_state.set_containers(metrics);
                    }
                }
                Err(err) => {
                    println!("Error on get_vm_cpu_and_mem: {:?}", err);
                }
            }
        }
    });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestApiModel {
    pub vms: BTreeMap<String, VmModel>,
    pub metrics: Option<Vec<MetricsByVm>>,
}

#[derive(Serialize, Deserialize)]
pub struct EnvsHttpModel {
    pub envs: Vec<String>,
    pub request_pass_key: bool,
}

#[server]
async fn get_envs() -> Result<EnvsHttpModel, ServerFnError> {
    let settings = crate::server::APP_CTX.settings_reader.get_settings().await;

    let user_id = dioxus::fullstack::FullstackContext::current()
        .and_then(|ctx| {
            let headers = ctx.parts_mut();
            headers
                .headers
                .get("x-ssl-user")
                .and_then(|user| user.to_str().ok())
                .map(|user| user.to_string())
        })
        .unwrap_or_default();

    let envs = settings.get_envs(&user_id);

    let mut request_pass_key = false;

    if settings.prompt_pass_phrase.unwrap_or(false) {
        if !crate::server::APP_CTX
            .ssh_private_key_resolver
            .private_key_is_loaded()
        {
            request_pass_key = true;
        }
    }
    Ok(EnvsHttpModel {
        envs,
        request_pass_key,
    })
}

#[server]
async fn get_vm_cpu_and_mem(
    env: String,
    selected_vm: String,
) -> Result<RequestApiModel, ServerFnError> {
    let cache_access_by_env = crate::server::APP_CTX.data_cache_by_env.lock().await;

    let cache_access = cache_access_by_env.envs.get(&env);

    if cache_access.is_none() {
        return Ok(RequestApiModel {
            vms: BTreeMap::new(),
            metrics: None,
        });
    }

    let cache_access = cache_access.unwrap();

    let vms = cache_access.get_vm_cpu_and_mem();

    let mut metrics = None;
    if !selected_vm.is_empty() {
        let selected_vm = crate::selected_vm::SelectedVm::from_string(selected_vm);
        let mut result = cache_access.get_metrics_by_vm(&selected_vm);

        for result in result.iter_mut() {
            if let Some(wrapper) = cache_access.metrics_history.get(&result.container.id) {
                result.container.cpu_usage_history = Some(wrapper.cpu.get_snapshot());
                result.container.mem_usage_history = Some(wrapper.mem.get_snapshot());
            }
        }

        metrics = Some(result);
    }

    Ok(RequestApiModel { vms, metrics })
}
