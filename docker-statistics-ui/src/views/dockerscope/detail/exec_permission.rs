use std::rc::Rc;

use dioxus::prelude::*;

use crate::api::ExecPermissionModel;
use crate::states::MainState;

/// Per-VM control for the time-limited unlock of the `exec_in_container` MCP tool.
///
/// The tool is disabled on every collector boot; a human opens a short window here,
/// and it closes itself. The window belongs to the VM that owns the container, so
/// this panel always addresses `vm_name` — unlocking one VM never opens exec anywhere
/// else. The UI exec console below is NOT affected: only the MCP (AI-agent) surface is.
#[component]
pub fn ExecPermissionPanel(vm_url: String, vm_name: String) -> Element {
    let env = consume_context::<Signal<MainState>>()
        .read()
        .envs
        .get_selected_env()
        .map(|e| e.clone())
        .unwrap_or_else(|| Rc::new(String::new()));

    let mut cs = use_signal(ExecPermissionState::default);

    // (Re)load whenever the selected VM changes.
    let env_for_load = env.clone();
    let url_for_load = vm_url.clone();
    let vm_for_load = vm_name.clone();
    let _loader = use_resource(use_reactive!(|vm_for_load| {
        let env = env_for_load.clone();
        let url = url_for_load.clone();
        async move {
            cs.write().begin_load();
            let result = crate::api::get_exec_permission(
                env.as_str().to_string(),
                url.clone(),
                vm_for_load.clone(),
            )
            .await;
            match result {
                Ok(model) => cs.write().set_loaded(model),
                Err(err) => cs.write().set_error(format!("{err:?}")),
            }
        }
    }));

    // Local 1s countdown. Cheaper and smoother than polling the api every second;
    // any real drift is corrected the next time the panel loads or a button is pressed.
    // peek() first so a closed window costs no write — and therefore no re-render.
    use_future(move || async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(1_000).await;
            if cs.peek().is_enabled() {
                cs.write().tick();
            }
        }
    });

    let cs_ra = cs.read();
    let status = cs_ra.status_text();
    let status_color = cs_ra.status_color();
    let is_enabled = cs_ra.is_enabled();
    let busy = cs_ra.loading;
    let error = cs_ra.error.clone();
    drop(cs_ra);

    let env_for_enable = env.clone();
    let url_for_enable = vm_url.clone();
    let vm_for_enable = vm_name.clone();

    let env_for_disable = env.clone();
    let url_for_disable = vm_url.clone();
    let vm_for_disable = vm_name.clone();

    rsx! {
        div { class: "panel",
            div { class: "panel-head",
                h3 { "MCP exec permission · {vm_name}" }
                span { class: "count-pill", style: "color: {status_color};", "{status}" }
            }
            div { class: "exec-perm-body",
                div { class: "exec-perm-hint",
                    "The "
                    code { "exec_in_container" }
                    " MCP tool lets an AI agent run arbitrary shell commands in containers on this VM. It is disabled by default — unlock it only while you need it."
                }
                div { class: "exec-perm-actions",
                    if is_enabled {
                        button {
                            class: "btn btn-sm btn-danger",
                            disabled: busy,
                            onclick: move |_| {
                                let (env, url, vm) = (
                                    env_for_disable.as_str().to_string(),
                                    url_for_disable.clone(),
                                    vm_for_disable.clone(),
                                );
                                spawn(async move {
                                    cs.write().begin_load();
                                    match crate::api::disable_exec_permission(env, url, vm).await {
                                        Ok(model) => cs.write().set_loaded(model),
                                        Err(err) => cs.write().set_error(format!("{err:?}")),
                                    }
                                });
                            },
                            "disable now"
                        }
                    } else {
                        button {
                            class: "btn btn-sm",
                            disabled: busy,
                            onclick: move |_| {
                                let (env, url, vm) = (
                                    env_for_enable.as_str().to_string(),
                                    url_for_enable.clone(),
                                    vm_for_enable.clone(),
                                );
                                spawn(async move {
                                    cs.write().begin_load();
                                    match crate::api::enable_exec_permission(env, url, vm).await {
                                        Ok(model) => cs.write().set_loaded(model),
                                        Err(err) => cs.write().set_error(format!("{err:?}")),
                                    }
                                });
                            },
                            "enable exec"
                        }
                    }
                }
            }
            if let Some(err) = error {
                div { class: "ds-error", "exec permission error: {err}" }
            }
        }
    }
}

#[derive(Default)]
struct ExecPermissionState {
    data: Option<ExecPermissionModel>,
    loading: bool,
    error: Option<String>,
}

impl ExecPermissionState {
    fn begin_load(&mut self) {
        self.loading = true;
        self.error = None;
    }

    fn set_loaded(&mut self, model: ExecPermissionModel) {
        self.data = Some(model);
        self.loading = false;
        self.error = None;
    }

    fn set_error(&mut self, err: String) {
        self.loading = false;
        self.error = Some(err);
    }

    /// Burns one second off the open window; closes it when it runs out.
    fn tick(&mut self) {
        let Some(data) = self.data.as_mut() else {
            return;
        };
        if !data.enabled {
            return;
        }
        data.seconds_left -= 1;
        if data.seconds_left <= 0 {
            data.seconds_left = 0;
            data.enabled = false;
        }
    }

    fn is_enabled(&self) -> bool {
        self.data.as_ref().map(|d| d.enabled).unwrap_or(false)
    }

    fn status_text(&self) -> String {
        if self.loading && self.data.is_none() {
            return "…".to_string();
        }
        match self.data.as_ref() {
            Some(d) if d.enabled => format!("● unlocked · {}", fmt_countdown(d.seconds_left)),
            Some(_) => "🔒 locked".to_string(),
            None => "unknown".to_string(),
        }
    }

    fn status_color(&self) -> &'static str {
        match self.data.as_ref() {
            Some(d) if d.enabled => "var(--warn)",
            _ => "var(--text-muted)",
        }
    }
}

fn fmt_countdown(seconds: i64) -> String {
    let seconds = seconds.max(0);
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
