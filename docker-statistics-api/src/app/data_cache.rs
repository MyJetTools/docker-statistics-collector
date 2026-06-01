use std::collections::{BTreeMap, HashMap};

use crate::{
    models::{ContainerJsonModel, ContainerModel, DiskModel, HostMemEntryModel, MetricsByVm, VmModel},
    selected_vm::SelectedVm,
};

use super::MetricsHistory;

#[derive(Clone)]
pub struct HostMemSnapshot {
    pub total: i64,
    pub available: i64,
    pub used: i64,
    pub cpu_count: Option<u32>,
    /// Host physical disks (empty when the host root filesystem isn't mounted).
    pub disks: Vec<DiskModel>,
}

pub struct MetricsHistoryWrapper {
    pub cpu: MetricsHistory<f64>,
    pub mem: MetricsHistory<i64>,
    pub open_files: MetricsHistory<i64>,
    pub net_in: MetricsHistory<f64>,
    pub net_out: MetricsHistory<f64>,
}
impl MetricsHistoryWrapper {
    pub fn new() -> Self {
        Self {
            cpu: MetricsHistory::new(),
            mem: MetricsHistory::new(),
            open_files: MetricsHistory::new(),
            net_in: MetricsHistory::new(),
            net_out: MetricsHistory::new(),
        }
    }
}

#[derive(Clone)]
pub struct ContainersWrapper {
    pub api_url: String,
    pub containers: BTreeMap<String, ContainerModel>,
    pub host_mem: Option<HostMemSnapshot>,
}

impl Into<ContainerModel> for ContainerJsonModel {
    fn into(self) -> ContainerModel {
        ContainerModel {
            id: self.id,
            image: self.image,
            names: self.names,
            labels: self.labels,
            enabled: self.enabled,
            created: self.created,
            started_at: self.started_at,
            state: self.state,
            status: self.status,
            instance: self.instance,
            cpu: self.cpu,
            mem: self.mem,
            files: self.files,
            net: self.net,
            cpu_usage_history: None,
            mem_usage_history: None,
            open_files_history: None,
            net_in_history: None,
            net_out_history: None,
            ports: self.ports,
            volumes: self.volumes,
        }
    }
}

pub struct DataCache {
    containers: BTreeMap<String, ContainersWrapper>,
    pub metrics_history: HashMap<String, MetricsHistoryWrapper>,
}

impl DataCache {
    pub fn new() -> Self {
        Self {
            containers: BTreeMap::new(),
            metrics_history: HashMap::new(),
        }
    }

    /// Replace this env's view with the freshly fanned-out master response.
    /// `containers_by_instance` already groups containers by their `instance`
    /// field (the source ENV_INFO of the collector each container comes from).
    /// Behaviour: VM buckets not present in this tick are pruned, so a peer
    /// that's currently down on the master simply disappears from the sidebar.
    pub fn update_from_master(
        &mut self,
        containers_by_instance: BTreeMap<String, Vec<ContainerJsonModel>>,
        host_mem_by_instance: HashMap<String, HostMemSnapshot>,
        master_url: String,
    ) {
        let active: std::collections::HashSet<String> =
            containers_by_instance.keys().cloned().collect();
        self.containers.retain(|vm, _| active.contains(vm));

        for (instance, containers) in containers_by_instance {
            let host_mem = host_mem_by_instance.get(&instance).cloned();
            self.update_one_vm(&instance, containers, host_mem, master_url.clone());
        }
    }

    fn update_one_vm(
        &mut self,
        vm: &str,
        containers: Vec<ContainerJsonModel>,
        host_mem: Option<HostMemSnapshot>,
        api_url: String,
    ) {
        let mut src = BTreeMap::new();

        for container in containers {
            src.insert(container.id.clone(), container);
        }

        if !self.containers.contains_key(vm) {
            self.containers.insert(
                vm.to_string(),
                ContainersWrapper {
                    api_url,
                    containers: BTreeMap::new(),
                    host_mem,
                },
            );
        } else {
            let w = self.containers.get_mut(vm).unwrap();
            w.api_url = api_url;
            w.host_mem = host_mem;
        }

        let by_vm = self.containers.get_mut(vm).unwrap();

        remove_not_used_keys_keys(&mut by_vm.containers, &src);

        for (id, container) in src {
            if let Some(usage) = container.cpu.usage {
                if !self.metrics_history.contains_key(&id) {
                    self.metrics_history
                        .insert(id.to_string(), MetricsHistoryWrapper::new());
                }

                let wrapper = self.metrics_history.get_mut(&id).unwrap();

                wrapper.cpu.add(usage);

                if let Some(usage) = container.mem.usage {
                    wrapper.mem.add(usage);
                }
            }

            if let Some(open) = container.files.open {
                if !self.metrics_history.contains_key(&id) {
                    self.metrics_history
                        .insert(id.to_string(), MetricsHistoryWrapper::new());
                }

                self.metrics_history
                    .get_mut(&id)
                    .unwrap()
                    .open_files
                    .add(open);
            }

            // Network throughput history — recorded whenever the collector
            // has a rate (i.e. after its second sample for the container).
            if container.net.in_mbps.is_some() || container.net.out_mbps.is_some() {
                if !self.metrics_history.contains_key(&id) {
                    self.metrics_history
                        .insert(id.to_string(), MetricsHistoryWrapper::new());
                }
                let wrapper = self.metrics_history.get_mut(&id).unwrap();
                wrapper.net_in.add(container.net.in_mbps.unwrap_or(0.0));
                wrapper.net_out.add(container.net.out_mbps.unwrap_or(0.0));
            }

            if !by_vm.containers.contains_key(&id) {
                by_vm.containers.insert(id.clone(), container.into());
            } else {
                let by_id = by_vm.containers.get_mut(&id).unwrap();
                by_id.update(container);
            }
        }
    }

    pub fn get_vm_cpu_and_mem(&self) -> BTreeMap<String, VmModel> {
        let mut result = BTreeMap::new();

        for (vm, wrapper) in self.containers.iter() {
            let mut cpu = 0.0;
            let mut mem = 0;
            let mut mem_limit = 0;
            let mut containers_amount = 0;
            let mut open_files = 0;
            let mut net_in_mbps = 0.0;
            let mut net_out_mbps = 0.0;

            // Effective limit for a container without `mem.limit` declared — it can
            // grab everything the host has, so we charge it the full host RAM.
            let unlimited_effective = wrapper.host_mem.as_ref().map(|s| s.total).unwrap_or(0);

            for itm in wrapper.containers.values() {
                if let Some(usage) = itm.cpu.usage {
                    cpu += usage;
                }

                if let Some(usage) = itm.mem.usage {
                    mem += usage;
                }

                match itm.mem.limit {
                    Some(v) if v > 0 => mem_limit += v,
                    _ => mem_limit += unlimited_effective,
                }

                if let Some(open) = itm.files.open {
                    open_files += open;
                }

                if let Some(v) = itm.net.in_mbps {
                    net_in_mbps += v;
                }
                if let Some(v) = itm.net.out_mbps {
                    net_out_mbps += v;
                }

                if itm.enabled {
                    containers_amount += 1;
                }
            }

            let (host_mem_total, host_mem_available, host_mem_used, host_cpu_count, host_disks) =
                match &wrapper.host_mem {
                    Some(snap) => (
                        Some(snap.total),
                        Some(snap.available),
                        Some(snap.used),
                        snap.cpu_count,
                        Some(snap.disks.clone()),
                    ),
                    None => (None, None, None, None, None),
                };

            result.insert(
                vm.clone(),
                VmModel {
                    api_url: wrapper.api_url.clone(),
                    cpu,
                    mem,
                    containers_amount,
                    mem_limit,
                    open_files,
                    net_in_mbps,
                    net_out_mbps,
                    host_mem_total,
                    host_mem_available,
                    host_mem_used,
                    host_cpu_count,
                    host_disks,
                },
            );
        }

        result
    }

    /// Helper for use_from_master callers — convert wire `HostMemEntryModel` to internal snapshot map.
    pub fn host_mem_map(entries: &[HostMemEntryModel]) -> HashMap<String, HostMemSnapshot> {
        entries
            .iter()
            .map(|e| {
                (
                    e.instance.clone(),
                    HostMemSnapshot {
                        total: e.total,
                        available: e.available,
                        used: e.used,
                        cpu_count: if e.cpu_count > 0 {
                            Some(e.cpu_count as u32)
                        } else {
                            None
                        },
                        disks: e.disks.clone(),
                    },
                )
            })
            .collect()
    }

    pub fn get_metrics_by_vm(&self, selected_vm: &SelectedVm) -> Vec<MetricsByVm> {
        match selected_vm {
            SelectedVm::All => {
                let mut result = Vec::new();

                for (vm, wrapper) in self.containers.iter() {
                    let host_total = wrapper.host_mem.as_ref().map(|s| s.total);
                    for itm in wrapper.containers.values() {
                        result.push(MetricsByVm {
                            vm: Some(vm.to_string()),
                            url: wrapper.api_url.clone(),
                            container: itm.clone(),
                            host_mem_total: host_total,
                        });
                    }
                }

                result
            }
            SelectedVm::SingleVm(vm) => match self.containers.get(vm) {
                Some(wrapper) => {
                    let mut result: Vec<MetricsByVm> = Vec::with_capacity(wrapper.containers.len());
                    let host_total = wrapper.host_mem.as_ref().map(|s| s.total);

                    for item in wrapper.containers.values() {
                        result.push(MetricsByVm {
                            vm: None,
                            url: wrapper.api_url.clone(),
                            container: item.clone(),
                            host_mem_total: host_total,
                        });
                    }

                    result
                }
                None => vec![],
            },
        }
    }
}

fn remove_not_used_keys_keys<TValue, TValue2>(
    current: &mut BTreeMap<String, TValue>,
    src: &BTreeMap<String, TValue2>,
) {
    let mut keys_to_removed = Vec::new();

    for key in current.keys() {
        if !src.contains_key(key) {
            keys_to_removed.push(key.to_string());
        }
    }

    for key_to_remove in keys_to_removed {
        current.remove(&key_to_remove);
    }
}
