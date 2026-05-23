use std::collections::BTreeMap;

use dioxus_shared::states::EnvListState;

use crate::{
    models::{MetricsByVm, VmModel},
    selected_vm::SelectedVm,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ContainerFilter {
    #[default]
    All,
    Running,
    Unhealthy,
    Restarting,
    Exited,
}

impl ContainerFilter {
    pub fn matches(&self, state: Option<&str>) -> bool {
        let s = state.map(|s| s.to_ascii_lowercase()).unwrap_or_default();
        match self {
            ContainerFilter::All => true,
            ContainerFilter::Running => s == "running",
            ContainerFilter::Unhealthy => s.contains("unhealthy"),
            ContainerFilter::Restarting => s == "restarting",
            ContainerFilter::Exited => s == "exited" || s == "dead" || s == "created",
        }
    }
}

pub struct MainState {
    pub envs: EnvListState,
    pub vms_state: Option<BTreeMap<String, VmModel>>,
    pub state_no: usize,
    pub data_request_no: i32,
    selected_vm: Option<SelectedVm>,
    containers: Option<Vec<MetricsByVm>>,
    filter: String,
    container_filter: ContainerFilter,
    active_container_name: Option<String>,

    pub dialog_is_shown: bool,
    pub prompt_pass_key: bool,
}

impl MainState {
    pub fn new() -> Self {
        Self {
            selected_vm: None,
            containers: None,
            filter: "".to_string(),
            container_filter: ContainerFilter::All,
            active_container_name: None,
            state_no: 0,
            dialog_is_shown: false,
            data_request_no: 0,
            vms_state: None,
            prompt_pass_key: false,
            envs: EnvListState::new(),
        }
    }

    pub fn set_selected_vm(&mut self, selected_vm: SelectedVm) {
        self.selected_vm = Some(selected_vm);
        self.containers = None;
        self.active_container_name = None;
        self.filter = String::new();
        self.container_filter = ContainerFilter::All;
        self.state_no += 1;
    }

    pub fn get_selected_vm_name(&self) -> Option<String> {
        match self.selected_vm.as_ref()? {
            SelectedVm::All => Some("***All***".to_string()),
            SelectedVm::SingleVm(v) => Some(v.clone()),
        }
    }

    pub fn get_container_filter(&self) -> ContainerFilter {
        self.container_filter
    }

    pub fn set_container_filter(&mut self, f: ContainerFilter) {
        self.container_filter = f;
    }

    pub fn get_active_container_name(&self) -> Option<&str> {
        self.active_container_name.as_deref()
    }

    pub fn set_active_container_name(&mut self, id: Option<String>) {
        self.active_container_name = id;
    }

    pub fn is_single_vm_selected(&self, vm: &str) -> bool {
        match self.selected_vm.as_ref() {
            Some(value) => {
                return value.is_single_selected_with_name(vm);
            }
            None => false,
        }
    }

    pub fn is_all_vms_selected(&self) -> bool {
        match self.selected_vm.as_ref() {
            Some(value) => {
                return value.is_all();
            }
            None => false,
        }
    }

    pub fn get_selected_vm(&self) -> (String, Option<SelectedVm>) {
        let selected_env = self.envs.get_selected_env().as_ref().unwrap().to_string();
        (selected_env, self.selected_vm.clone())
    }

    pub fn get_containers(&self) -> Option<Vec<&MetricsByVm>> {
        let items = self.containers.as_ref()?;

        let mut result = Vec::with_capacity(items.len());
        for itm in items.iter() {
            if !itm.container.filter_me(&self.filter) {
                continue;
            }
            if !self.container_filter.matches(itm.container.state.as_deref()) {
                continue;
            }
            result.push(itm)
        }

        result.sort_by(|a, b| a.container.image.cmp(&b.container.image));

        Some(result)
    }

    pub fn get_all_containers(&self) -> Option<&Vec<MetricsByVm>> {
        self.containers.as_ref()
    }

    pub fn set_containers(&mut self, containers: Vec<MetricsByVm>) {
        // Don't auto-drop the active selection when it's not in the new list —
        // the router is the source of truth; data may simply not have arrived
        // yet for that VM/env. Detail panel just renders the empty state.
        self.containers = Some(containers);
    }

    pub fn find_active_container(&self) -> Option<&MetricsByVm> {
        let name = self.active_container_name.as_deref()?;
        self.containers
            .as_ref()?
            .iter()
            .find(|c| primary_name(&c.container.names).eq_ignore_ascii_case(name))
    }

    pub fn set_filter(&mut self, value: String) {
        self.filter = value;
    }

    pub fn get_filter(&self) -> &str {
        &self.filter
    }
}

pub fn primary_name(names: &[String]) -> &str {
    names
        .first()
        .map(|n| n.trim_start_matches('/'))
        .unwrap_or("")
}
