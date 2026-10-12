use std::collections::{BTreeMap, HashMap};

use rust_extensions::date_time::DateTimeAsMicroseconds;

use crate::{
    models::{
        ContainerJsonModel, ContainerModel, DiskModel, HostMemEntryModel, MetricsByVm,
        NetSample, NetUsageJsonMode, VmModel,
    },
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

/// First sighting of a container in a VM bucket. `net` is the throughput this service
/// derived — still `None` on the very first poll, since one sample cannot be a rate.
fn to_container_model(src: ContainerJsonModel, net: NetUsageJsonMode) -> ContainerModel {
    let net_prev = src.net.as_sample();
    let started_at = src.started_at_or_none();
    ContainerModel {
        id: src.id,
        image: src.image,
        names: src.names,
        labels: src.labels,
        enabled: src.enabled,
        created: src.created,
        started_at,
        state: src.state,
        status: src.status,
        instance: src.instance,
        cpu: src.cpu,
        mem: src.mem,
        files: src.files,
        net,
        net_prev,
        disk: src.disk,
        cpu_usage_history: None,
        mem_usage_history: None,
        open_files_history: None,
        net_in_history: None,
        net_out_history: None,
        ports: src.ports,
        volumes: src.volumes,
    }
}

/// How many consecutive ticks a VM may be missing from the master's answer before its
/// bucket is dropped. A peer now has to complete a full live Docker scan inside the
/// master's peer timeout, so a single miss is a routine blip rather than evidence the
/// host is gone — and dropping the bucket takes the whole VM off the operator's screen
/// during exactly the incident they opened the page for.
const MISSING_TICKS_BEFORE_PRUNE: u32 = 4;

/// Per-container state that must outlive its VM bucket.
///
/// `net_prev` is the anchor the next throughput reading is derived from, and it exists
/// nowhere else — the collector forwards raw counters and remembers nothing. A bucket
/// that is pruned and re-created would otherwise lose a net sample per container.
/// (`disk` needs no such treatment: the collector measures it on its own timer and
/// every payload carries it, so a re-created bucket is repopulated on the next poll.)
#[derive(Clone, Default)]
struct ContainerSideState {
    net_prev: Option<NetSample>,
}

pub struct DataCache {
    containers: BTreeMap<String, ContainersWrapper>,
    pub metrics_history: HashMap<String, MetricsHistoryWrapper>,
    /// Keyed by container id, independent of which VM bucket currently holds it.
    side_state: HashMap<String, ContainerSideState>,
    /// Consecutive ticks each known VM has been absent from the master's answer.
    missing_ticks: HashMap<String, u32>,
    /// When this env last produced a usable answer. Nothing recorded staleness before,
    /// so a frozen dashboard was indistinguishable from a quiet fleet.
    last_successful_poll_at: Option<DateTimeAsMicroseconds>,
}

impl DataCache {
    pub fn new() -> Self {
        Self {
            containers: BTreeMap::new(),
            metrics_history: HashMap::new(),
            side_state: HashMap::new(),
            missing_ticks: HashMap::new(),
            last_successful_poll_at: None,
        }
    }

    /// Unix microseconds of the last successful poll of this env, for staleness display.
    pub fn last_successful_poll_at(&self) -> Option<DateTimeAsMicroseconds> {
        self.last_successful_poll_at
    }

    /// Merge the freshly fanned-out master response into this env's view.
    /// `containers_by_instance` already groups containers by their `instance`
    /// field (the source ENV_INFO of the collector each container comes from).
    ///
    /// A VM missing from this tick is NOT dropped straight away — see
    /// [`MISSING_TICKS_BEFORE_PRUNE`]. It keeps its last known containers so a peer
    /// that merely answered slowly stays on the rail, and only a host that is
    /// persistently absent disappears.
    pub fn update_from_master(
        &mut self,
        containers_by_instance: BTreeMap<String, Vec<ContainerJsonModel>>,
        host_mem_by_instance: HashMap<String, HostMemSnapshot>,
        master_url: String,
    ) {
        let active: std::collections::HashSet<String> =
            containers_by_instance.keys().cloned().collect();

        let mut to_prune = Vec::new();
        for vm in self.containers.keys() {
            if active.contains(vm) {
                continue;
            }
            let missed = self.missing_ticks.entry(vm.clone()).or_insert(0);
            *missed += 1;
            if *missed >= MISSING_TICKS_BEFORE_PRUNE {
                to_prune.push(vm.clone());
            }
        }

        for vm in to_prune {
            if let Some(wrapper) = self.containers.remove(&vm) {
                // The host really is gone — now the side state for its containers is
                // dead weight rather than something worth preserving.
                for id in wrapper.containers.keys() {
                    self.side_state.remove(id);
                    self.metrics_history.remove(id);
                }
            }
            self.missing_ticks.remove(&vm);
        }

        for instance in active.iter() {
            self.missing_ticks.remove(instance);
        }

        for (instance, containers) in containers_by_instance {
            let host_mem = host_mem_by_instance.get(&instance).cloned();
            self.update_one_vm(&instance, containers, host_mem, master_url.clone());
        }

        self.last_successful_poll_at = Some(DateTimeAsMicroseconds::now());
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

        let mut gone: Vec<String> = Vec::new();
        let by_vm = self.containers.get_mut(vm).unwrap();

        // A container gone from a VM that DID answer is genuinely gone — drop its side
        // state now rather than leaking it. (A VM that failed to answer never reaches
        // here, so a blip cannot trigger this.)
        for id in by_vm.containers.keys() {
            if !src.contains_key(id) {
                gone.push(id.clone());
            }
        }
        remove_not_used_keys_keys(&mut by_vm.containers, &src);

        for (id, container) in src {
            // The collector ships raw counters; the rate lives here, where the previous
            // reading is remembered. The anchor comes from the side state rather than
            // the bucket, so it survives a VM that blipped out for a tick.
            let side = self.side_state.entry(id.clone()).or_default();
            let net = derive_net_rate(side.net_prev.as_ref(), &container);
            // Only overwrite with a USABLE reading. When the collector could not read a
            // container's stats it ships no counters, and clearing the anchor on that
            // would mean a container whose /stats fails every other poll never shows
            // throughput at all — a rate across a longer gap is still a correct rate.
            if let Some(sample) = container.net.as_sample() {
                side.net_prev = Some(sample);
            }

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

            // Network throughput history — recorded once a rate exists, i.e.
            // from the second poll of a container onwards.
            if net.in_mbps.is_some() || net.out_mbps.is_some() {
                if !self.metrics_history.contains_key(&id) {
                    self.metrics_history
                        .insert(id.to_string(), MetricsHistoryWrapper::new());
                }
                let wrapper = self.metrics_history.get_mut(&id).unwrap();
                wrapper.net_in.add(net.in_mbps.unwrap_or(0.0));
                wrapper.net_out.add(net.out_mbps.unwrap_or(0.0));
            }

            if !by_vm.containers.contains_key(&id) {
                by_vm
                    .containers
                    .insert(id.clone(), to_container_model(container, net));
            } else {
                let by_id = by_vm.containers.get_mut(&id).unwrap();
                by_id.update(container, net);
            }
        }

        for id in gone {
            self.side_state.remove(&id);
            self.metrics_history.remove(&id);
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

/// Turn the collector's raw counters into MB/s using the reading kept from the
/// previous poll. `None`/`None` until two readings exist for the container.
fn derive_net_rate(previous: Option<&NetSample>, incoming: &ContainerJsonModel) -> NetUsageJsonMode {
    let (Some(prev), Some(next)) = (previous, incoming.net.as_sample()) else {
        return NetUsageJsonMode::default();
    };

    prev.rate_to(&next).unwrap_or_default()
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
