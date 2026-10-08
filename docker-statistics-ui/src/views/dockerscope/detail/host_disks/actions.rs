use std::collections::BTreeMap;

use crate::models::{DiskModel, VmModel};
use crate::views::dockerscope::helpers::DiskSeverity;

/// One physical filesystem of the Host disks board — everything the row needs,
/// already flattened out of `VmModel` so the render pass does no lookups.
#[derive(Clone, PartialEq)]
pub struct HostDiskRow {
    /// Host the filesystem belongs to. Always known, unlike a container row's
    /// `vm`: disks are read from the VM map, which is keyed by it.
    pub vm: String,
    pub mount_point: String,
    /// Name an operator gave the disk; the row leads with it when there is one.
    pub title: Option<String>,
    pub device: String,
    pub fs_type: String,
    pub total: i64,
    pub used: i64,
    pub available: i64,
    /// Percent of the filesystem in use — the figure the row is ranked on, and
    /// the same one it shows as its headline.
    pub used_pct: f64,
    pub severity: DiskSeverity,
}

impl HostDiskRow {
    fn from(vm: &str, disk: &DiskModel) -> Self {
        let used_pct = disk.used_pct();
        Self {
            vm: vm.to_string(),
            mount_point: disk.mount_point.clone(),
            title: disk.title.clone(),
            device: disk.device.clone(),
            fs_type: disk.fs_type.clone(),
            total: disk.total,
            used: disk.used,
            available: disk.available,
            used_pct,
            severity: DiskSeverity::from_used_pct(used_pct),
        }
    }
}

/// The ranked board plus the aggregates its header is drawn from.
pub struct HostDisks {
    pub rows: Vec<HostDiskRow>,
    /// How many disks were ranked before the Top-N cut.
    pub ranked: usize,
    /// Disks at or above the warn level, counted over **every** disk ranked and
    /// not just the rows kept — a cut that hides a hot disk still shows up in
    /// the header.
    pub hot: usize,
    /// Hosts in scope that reported no disk at all. They are missing from the
    /// board, which says nothing about how full they are.
    pub hosts_without_disks: usize,
}

/// Rank the physical disks of `single_vm` — or of every host when it is `None` —
/// by how full they are, keeping the top `top_n` (`0` keeps all).
///
/// Unlike the container boards this ignores the search box and the state chips:
/// those filter containers, and a disk is not one.
pub fn build_host_disks(
    vms: &BTreeMap<String, VmModel>,
    single_vm: Option<&str>,
    top_n: usize,
) -> HostDisks {
    let mut rows = Vec::new();
    let mut hosts_without_disks = 0;

    for (name, vm) in vms {
        if single_vm.is_some_and(|selected| selected != name.as_str()) {
            continue;
        }

        // `None` (the collector reported no host block) and an empty list (it
        // could not measure a single filesystem) read the same from here.
        let disks = vm.host_disks.as_deref().unwrap_or_default();
        if disks.is_empty() {
            hosts_without_disks += 1;
            continue;
        }

        rows.extend(disks.iter().map(|disk| HostDiskRow::from(name, disk)));
    }

    let ranked = rows.len();
    let hot = rows
        .iter()
        .filter(|row| row.severity != DiskSeverity::Ok)
        .count();

    // Fullest first, by percent rather than by bytes: a disk is in trouble by how
    // little of it is left, not by how much it holds. Host and mount point are a
    // tiebreak only, so equally full disks keep their order between polls.
    rows.sort_by(|a, b| {
        b.used_pct
            .total_cmp(&a.used_pct)
            .then_with(|| a.vm.cmp(&b.vm))
            .then_with(|| a.mount_point.cmp(&b.mount_point))
    });

    if top_n > 0 && rows.len() > top_n {
        rows.truncate(top_n);
    }

    HostDisks {
        rows,
        ranked,
        hot,
        hosts_without_disks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: i64 = 1024 * 1024 * 1024;

    fn disk(mount_point: &str, total_gib: i64, used_gib: i64) -> DiskModel {
        DiskModel {
            device: format!("/dev{}", mount_point),
            mount_point: mount_point.to_string(),
            fs_type: "ext4".to_string(),
            total: total_gib * GIB,
            used: used_gib * GIB,
            available: (total_gib - used_gib) * GIB,
            title: None,
        }
    }

    fn vm(host_disks: Option<Vec<DiskModel>>) -> VmModel {
        VmModel {
            api_url: String::new(),
            cpu: 0.0,
            mem: 0,
            mem_limit: 0,
            containers_amount: 0,
            open_files: 0,
            net_in_mbps: 0.0,
            net_out_mbps: 0.0,
            host_mem_total: None,
            host_mem_available: None,
            host_mem_used: None,
            host_cpu_count: None,
            host_disks,
        }
    }

    fn fleet() -> BTreeMap<String, VmModel> {
        let mut vms = BTreeMap::new();
        vms.insert(
            "app".to_string(),
            vm(Some(vec![disk("/", 100, 80), disk("/data", 1000, 300)])),
        );
        vms.insert("db".to_string(), vm(Some(vec![disk("/", 100, 95)])));
        // Host root not mounted into the collector, and no host block at all.
        vms.insert("ci".to_string(), vm(Some(Vec::new())));
        vms.insert("old".to_string(), vm(None));
        vms
    }

    fn order(board: &HostDisks) -> Vec<(&str, &str)> {
        board
            .rows
            .iter()
            .map(|row| (row.vm.as_str(), row.mount_point.as_str()))
            .collect()
    }

    #[test]
    fn the_fleet_is_ranked_fullest_first_by_percent_not_by_bytes() {
        let board = build_host_disks(&fleet(), None, 0);
        // `/data` holds the most bytes (300G) and is still last: it is the emptiest.
        assert_eq!(order(&board), vec![("db", "/"), ("app", "/"), ("app", "/data")]);
        assert_eq!(board.rows[0].severity, DiskSeverity::Danger);
        assert_eq!(board.rows[1].severity, DiskSeverity::Warn);
        assert_eq!(board.rows[2].severity, DiskSeverity::Ok);
    }

    #[test]
    fn hosts_that_report_no_disks_are_counted_not_dropped_silently() {
        let board = build_host_disks(&fleet(), None, 0);
        assert_eq!(board.hosts_without_disks, 2);
    }

    #[test]
    fn a_single_vm_shows_only_its_own_disks() {
        let board = build_host_disks(&fleet(), Some("app"), 0);
        assert_eq!(order(&board), vec![("app", "/"), ("app", "/data")]);
        assert_eq!(board.hosts_without_disks, 0);

        let board = build_host_disks(&fleet(), Some("ci"), 0);
        assert!(board.rows.is_empty());
        assert_eq!(board.ranked, 0);
    }

    #[test]
    fn the_cut_keeps_the_fullest_and_the_header_still_counts_the_whole_set() {
        let board = build_host_disks(&fleet(), None, 1);
        assert_eq!(order(&board), vec![("db", "/")]);
        assert_eq!(board.ranked, 3);
        // `app:/` is hot too — hidden by the cut, but not from the count.
        assert_eq!(board.hot, 2);
    }

    #[test]
    fn equally_full_disks_keep_a_stable_order() {
        let mut vms = BTreeMap::new();
        vms.insert("b".to_string(), vm(Some(vec![disk("/", 10, 5)])));
        vms.insert(
            "a".to_string(),
            vm(Some(vec![disk("/var", 20, 10), disk("/", 40, 20)])),
        );
        let board = build_host_disks(&vms, None, 0);
        assert_eq!(order(&board), vec![("a", "/"), ("a", "/var"), ("b", "/")]);
    }
}
