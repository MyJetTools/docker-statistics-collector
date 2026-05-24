use std::collections::BTreeMap;
use std::sync::Arc;

use my_http_server::{
    macros::{http_route, MyHttpInput},
    HttpContext, HttpFailResult, HttpOkResult, HttpOutput,
};

use crate::app::AppCtx;
use crate::models::RequestApiModel;
use crate::selected_vm::SelectedVm;

#[http_route(
    method: "GET",
    route: "/api/vm_cpu_and_mem",
    controller: "Metrics",
    description: "Returns aggregated VM metrics for the env, plus per-container metrics when selected_vm is set",
    summary: "VM and container metrics snapshot",
    input_data: GetVmCpuAndMemInputModel,
    result:[
        {status_code: 200, description: "RequestApiModel JSON"},
    ]
)]
pub struct GetVmCpuAndMemAction {
    app: Arc<AppCtx>,
}

impl GetVmCpuAndMemAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

#[derive(MyHttpInput)]
pub struct GetVmCpuAndMemInputModel {
    #[http_query(name = "env", description = "Environment name")]
    pub env: String,

    #[http_query(name = "selected_vm", description = "Selected VM name (empty = none)")]
    pub selected_vm: String,
}

async fn handle_request(
    action: &GetVmCpuAndMemAction,
    input_data: GetVmCpuAndMemInputModel,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let user_id = crate::auth::user_from_http(ctx);
    let settings = action.app.settings_reader.get_settings().await;
    if !settings.is_env_allowed_for_user(&user_id, &input_data.env) {
        return Err(HttpFailResult::as_forbidden(Some(format!(
            "env '{}' is not accessible for user '{}'",
            input_data.env, user_id
        ))));
    }
    drop(settings);

    let cache_access_by_env = action.app.data_cache_by_env.lock().await;

    let cache_access = cache_access_by_env.envs.get(&input_data.env);

    let response = match cache_access {
        None => RequestApiModel {
            vms: BTreeMap::new(),
            metrics: None,
        },
        Some(cache) => {
            let vms = cache.get_vm_cpu_and_mem();

            let mut metrics = None;
            if !input_data.selected_vm.is_empty() {
                let selected_vm = SelectedVm::from_string(input_data.selected_vm);
                let mut result = cache.get_metrics_by_vm(&selected_vm);

                for row in result.iter_mut() {
                    if let Some(wrapper) = cache.metrics_history.get(&row.container.id) {
                        row.container.cpu_usage_history = Some(wrapper.cpu.get_snapshot());
                        row.container.mem_usage_history = Some(wrapper.mem.get_snapshot());
                        row.container.open_files_history =
                            Some(wrapper.open_files.get_snapshot());
                    }
                }

                metrics = Some(result);
            }

            RequestApiModel { vms, metrics }
        }
    };

    HttpOutput::as_json(response)
        .with_compression(1024)
        .into_ok_result(false)
        .into()
}
