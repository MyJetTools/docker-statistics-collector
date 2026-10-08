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

/// [`fmt_mem_pair`] with one more step down, for disk sizes. Memory never needs
/// it, but a container's writable layer is routinely a few kilobytes — in MiB a
/// board of them reads `0` against bars of visibly different lengths.
pub fn fmt_disk_pair(bytes: i64) -> (String, &'static str) {
    if bytes >= 1024 * 1024 {
        return fmt_mem_pair(bytes);
    }
    (format!("{:.0}", bytes.max(0) as f64 / 1024.0), "KiB")
}

/// Tooltip spelling out both disk figures of a container. Shared by the container
/// list and the disk board, so the two cannot word the same numbers differently.
pub fn disk_size_title(size_rw: Option<i64>, size_root_fs: Option<i64>) -> String {
    match (size_rw, size_root_fs) {
        (Some(rw), Some(root)) => format!(
            "writable layer {} · total with image {}",
            fmt_mem_short(rw),
            fmt_mem_short(root)
        ),
        (Some(rw), _) => format!("writable layer {}", fmt_mem_short(rw)),
        _ => "disk size not measured yet".to_string(),
    }
}

/// Fill level, in percent, from which a host disk is [`DiskSeverity::Warn`].
pub const DISK_WARN_PCT: f64 = 75.0;
/// Fill level, in percent, from which a host disk is [`DiskSeverity::Danger`].
pub const DISK_DANGER_PCT: f64 = 90.0;

/// Host-disk severity by fill level. One definition for the VM rail and the Host
/// disks board, so a filesystem cannot be amber in one column and calm in the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskSeverity {
    Ok,
    Warn,
    Danger,
}

impl DiskSeverity {
    pub fn from_used_pct(used_pct: f64) -> Self {
        if used_pct >= DISK_DANGER_PCT {
            DiskSeverity::Danger
        } else if used_pct >= DISK_WARN_PCT {
            DiskSeverity::Warn
        } else {
            DiskSeverity::Ok
        }
    }

    /// Modifier class for the bar's fill — `.vm-disk-bar-used` and `.tc-fill` both
    /// style these two names. `Ok` adds nothing: the bar keeps its own colour.
    pub fn fill_class(&self) -> &'static str {
        match self {
            DiskSeverity::Danger => "col-danger",
            DiskSeverity::Warn => "col-warn",
            DiskSeverity::Ok => "",
        }
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
        // Aggregate "All VMs" card intentionally hides per-disk usage.
        host_disks: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_sizes_step_down_to_kib_and_match_memory_above_it() {
        assert_eq!(fmt_disk_pair(0), ("0".to_string(), "KiB"));
        assert_eq!(fmt_disk_pair(12 * 1024), ("12".to_string(), "KiB"));
        assert_eq!(fmt_disk_pair(340 * 1024 * 1024), ("340".to_string(), "MiB"));
        assert_eq!(
            fmt_disk_pair(3 * 1024 * 1024 * 1024 / 2),
            fmt_mem_pair(3 * 1024 * 1024 * 1024 / 2)
        );
    }

    #[test]
    fn disk_severity_turns_at_the_two_thresholds() {
        assert_eq!(DiskSeverity::from_used_pct(74.9), DiskSeverity::Ok);
        assert_eq!(DiskSeverity::from_used_pct(DISK_WARN_PCT), DiskSeverity::Warn);
        assert_eq!(DiskSeverity::from_used_pct(89.9), DiskSeverity::Warn);
        assert_eq!(DiskSeverity::from_used_pct(DISK_DANGER_PCT), DiskSeverity::Danger);
    }

    #[test]
    fn a_disk_tooltip_only_claims_what_was_measured() {
        assert_eq!(
            disk_size_title(Some(2 * 1024 * 1024), Some(5 * 1024 * 1024)),
            "writable layer 2M · total with image 5M"
        );
        assert_eq!(disk_size_title(Some(2 * 1024 * 1024), None), "writable layer 2M");
        assert_eq!(disk_size_title(None, None), "disk size not measured yet");
    }
}
