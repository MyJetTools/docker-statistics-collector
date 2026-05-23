use std::collections::BTreeMap;

use super::EnvListState;

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
    pub vms_state: BTreeMap<String, VmModel>,
    pub state_no: usize,
    pub data_request_no: i32,
    selected_vm: Option<SelectedVm>,
    containers: Vec<MetricsByVm>,
    filter: String,
    container_filter: ContainerFilter,
    active_container_name: Option<String>,
    /// VM name of the active container. Required to disambiguate when the same
    /// container name lives on multiple VMs in `/all` view. In single-VM view
    /// it's also populated (with the selected VM) — `find_active_container`
    /// degrades gracefully when row.vm is None.
    active_container_vm: Option<String>,

    pub dialog_is_shown: bool,
    pub prompt_pass_key: bool,
}

impl MainState {
    pub fn new() -> Self {
        Self {
            selected_vm: None,
            containers: Vec::new(),
            filter: "".to_string(),
            container_filter: ContainerFilter::All,
            active_container_name: None,
            active_container_vm: None,
            state_no: 0,
            dialog_is_shown: false,
            data_request_no: 0,
            vms_state: BTreeMap::new(),
            prompt_pass_key: false,
            envs: EnvListState::new(),
        }
    }

    pub fn set_selected_vm(&mut self, selected_vm: SelectedVm) {
        self.selected_vm = Some(selected_vm);
        self.containers = Vec::new();
        self.active_container_name = None;
        self.active_container_vm = None;
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

    pub fn get_active_container_vm(&self) -> Option<&str> {
        self.active_container_vm.as_deref()
    }

    pub fn set_active_container(&mut self, name: Option<String>, vm: Option<String>) {
        self.active_container_name = name;
        self.active_container_vm = vm;
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

    pub fn get_containers(&self) -> Vec<&MetricsByVm> {
        let mut result = Vec::with_capacity(self.containers.len());
        for itm in self.containers.iter() {
            if !itm.container.filter_me(&self.filter) {
                continue;
            }
            if !self.container_filter.matches(itm.container.state.as_deref()) {
                continue;
            }
            result.push(itm)
        }

        result.sort_by(|a, b| a.container.image.cmp(&b.container.image));

        result
    }

    pub fn get_all_containers(&self) -> &Vec<MetricsByVm> {
        &self.containers
    }

    pub fn set_containers(&mut self, containers: Vec<MetricsByVm>) {
        // Don't auto-drop the active selection when it's not in the new list —
        // the router is the source of truth; data may simply not have arrived
        // yet for that VM/env. Detail panel just renders the empty state.
        self.containers = containers;
    }

    pub fn find_active_container(&self) -> Option<&MetricsByVm> {
        let name = self.active_container_name.as_deref()?;
        let target_vm = self.active_container_vm.as_deref();
        self.containers.iter().find(|c| {
            if !primary_name(&c.container.names).eq_ignore_ascii_case(name) {
                return false;
            }
            match (target_vm, c.vm.as_deref()) {
                (Some(tv), Some(rv)) => tv.eq_ignore_ascii_case(rv),
                // Single-VM view: rows carry vm=None; the selected VM is implicit.
                (Some(_), None) => true,
                (None, _) => true,
            }
        })
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
