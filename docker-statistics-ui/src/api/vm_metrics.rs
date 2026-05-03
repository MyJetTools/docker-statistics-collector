use std::collections::BTreeMap;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::{MetricsByVm, VmModel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestApiModel {
    pub vms: BTreeMap<String, VmModel>,
    pub metrics: Option<Vec<MetricsByVm>>,
}

#[get("/api/vm_cpu_and_mem?env&selected_vm")]
pub async fn get_vm_cpu_and_mem(
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
