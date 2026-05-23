use std::collections::BTreeMap;

use crate::models::VmModel;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VmGroup {
    Production,
    Staging,
    Dev,
}

impl VmGroup {
    pub fn classify(name: &str) -> Self {
        let n = name.to_ascii_lowercase();
        if n.contains("prod") {
            VmGroup::Production
        } else if n.contains("stage") {
            VmGroup::Staging
        } else {
            VmGroup::Dev
        }
    }
}

pub struct GroupedVms<'a> {
    pub production: Vec<(&'a String, &'a VmModel)>,
    pub staging: Vec<(&'a String, &'a VmModel)>,
    pub dev: Vec<(&'a String, &'a VmModel)>,
}

pub fn group_vms<'a>(vms: &'a BTreeMap<String, VmModel>) -> GroupedVms<'a> {
    let mut production = Vec::new();
    let mut staging = Vec::new();
    let mut dev = Vec::new();
    for (name, vm) in vms {
        match VmGroup::classify(name) {
            VmGroup::Production => production.push((name, vm)),
            VmGroup::Staging => staging.push((name, vm)),
            VmGroup::Dev => dev.push((name, vm)),
        }
    }
    GroupedVms { production, staging, dev }
}

pub fn fmt_mem_short(bytes: i64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1}G", mb / 1024.0)
    } else if mb >= 1.0 {
        format!("{:.0}M", mb)
    } else {
        let kb = bytes as f64 / 1024.0;
        format!("{:.0}K", kb)
    }
}

pub fn fmt_mem_pair(bytes: i64) -> (String, &'static str) {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        (format!("{:.2}", mb / 1024.0), "GiB")
    } else {
        (format!("{:.0}", mb.max(0.0)), "MiB")
    }
}

pub fn pct(numer: i64, denom: i64) -> f64 {
    if denom <= 0 {
        0.0
    } else {
        ((numer as f64) / (denom as f64) * 100.0).clamp(0.0, 100.0)
    }
}

/// "ok" | "warn" | "danger" — mirrors the prototype's `vm.status` field, derived from VM load.
pub fn vm_status(vm: &VmModel) -> &'static str {
    let mem_pct = pct(vm.mem, vm.mem_limit);
    let cpu = vm.cpu;
    if cpu >= 85.0 || mem_pct >= 90.0 {
        "danger"
    } else if cpu >= 65.0 || mem_pct >= 75.0 {
        "warn"
    } else {
        "ok"
    }
}

pub fn state_class_for(state: Option<&str>) -> &'static str {
    let s = state.map(|x| x.to_ascii_lowercase()).unwrap_or_default();
    if s == "running" {
        ""
    } else if s == "restarting" {
        "restarting"
    } else if s.contains("unhealthy") {
        "unhealthy"
    } else {
        "exited"
    }
}

pub fn shorten_id(id: &str, n: usize) -> &str {
    if id.len() <= n {
        id
    } else {
        &id[..n]
    }
}
