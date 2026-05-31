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

/// VM-card memory severity, computed from used / reserved (sum of limits) / host total.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MemSeverity {
    Ok,
    Warn,
    Danger,
}

/// Severity rules:
/// - `Danger` if reserved > host_total (over-committed: containers can claim more than VM has)
///            OR used / host_total >= 90%.
/// - `Warn`   if reserved / host_total >= 80%
///            OR used / host_total >= 75%.
/// - `Ok`     otherwise.
/// host_total = None → severity is based only on used vs reserved (Warn if used >= 90% of reserved).
pub fn vm_mem_severity(used: i64, reserved: i64, host_total: Option<i64>) -> MemSeverity {
    if let Some(total) = host_total {
        if total > 0 {
            if reserved > total {
                return MemSeverity::Danger;
            }
            let used_pct = pct(used, total);
            let reserved_pct = pct(reserved, total);
            if used_pct >= 90.0 {
                return MemSeverity::Danger;
            }
            if reserved_pct >= 80.0 || used_pct >= 75.0 {
                return MemSeverity::Warn;
            }
            return MemSeverity::Ok;
        }
    }
    if reserved > 0 && pct(used, reserved) >= 90.0 {
        MemSeverity::Warn
    } else {
        MemSeverity::Ok
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

/// Auto-scale a byte count into a `(value, unit)` pair, mirroring MyNoSqlServer's
/// `format_bytes` convention (TypeScript/Utils.ts): binary (1024) steps, 2-decimal
/// precision and short `b / Kb / Mb / Gb / Tb` suffixes.
pub fn format_bytes_pair(bytes: f64) -> (String, &'static str) {
    const UNITS: [&str; 5] = ["b", "Kb", "Mb", "Gb", "Tb"];
    let mut value = bytes.max(0.0);
    let mut idx = 0;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    (format!("{:.2}", value), UNITS[idx])
}

/// Auto-scale a bytes-per-second rate (input is MB/s, i.e. MiB/s — the unit the
/// collector reports) into a `(value, unit)` pair that picks the most readable
/// magnitude. Reuses [`format_bytes_pair`] so the suffixes match the rest of the
/// fleet (`B/s`, `KB/s`, `MB/s`, `GB/s`).
pub fn fmt_throughput_pair(mbps: f64) -> (String, String) {
    let bytes_per_sec = mbps.max(0.0) * 1024.0 * 1024.0;
    let (v, u) = format_bytes_pair(bytes_per_sec);
    (v, format!("{}/s", u))
}

/// Single-string form of [`fmt_throughput_pair`], e.g. `"12.34 MB/s"`.
pub fn fmt_throughput(mbps: f64) -> String {
    let (v, u) = fmt_throughput_pair(mbps);
    format!("{} {}", v, u)
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

/// Synthesize a single VmModel from the fleet so the "All VMs" rail card can
/// reuse the regular VmCard layout. Sums numeric fields; host_* totals sum
/// only across VMs that report them.
pub fn aggregate_all_vms(vms: &BTreeMap<String, VmModel>) -> VmModel {
    let mut cpu = 0.0_f64;
    let mut mem = 0_i64;
    let mut mem_limit = 0_i64;
    let mut containers_amount = 0_usize;
    let mut open_files = 0_i64;
    let mut net_in_mbps = 0.0_f64;
    let mut net_out_mbps = 0.0_f64;
    let mut host_mem_total: Option<i64> = None;
    let mut host_mem_available: Option<i64> = None;
    let mut host_mem_used: Option<i64> = None;
    let mut host_cpu_count: Option<u32> = None;

    for vm in vms.values() {
        cpu += vm.cpu;
        mem += vm.mem;
        mem_limit += vm.mem_limit;
        containers_amount += vm.containers_amount;
        open_files += vm.open_files;
        net_in_mbps += vm.net_in_mbps;
        net_out_mbps += vm.net_out_mbps;
        if let Some(t) = vm.host_mem_total {
            host_mem_total = Some(host_mem_total.unwrap_or(0) + t);
        }
        if let Some(a) = vm.host_mem_available {
            host_mem_available = Some(host_mem_available.unwrap_or(0) + a);
        }
        if let Some(u) = vm.host_mem_used {
            host_mem_used = Some(host_mem_used.unwrap_or(0) + u);
        }
        if let Some(c) = vm.host_cpu_count {
            host_cpu_count = Some(host_cpu_count.unwrap_or(0) + c);
        }
    }

    VmModel {
        api_url: String::new(),
        cpu,
        mem,
        mem_limit,
        containers_amount,
        open_files,
        net_in_mbps,
        net_out_mbps,
        host_mem_total,
        host_mem_available,
        host_mem_used,
        host_cpu_count,
    }
}
