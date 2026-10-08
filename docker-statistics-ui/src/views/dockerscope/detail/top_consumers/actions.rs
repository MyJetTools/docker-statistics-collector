use crate::models::MetricsByVm;
use crate::states::{primary_name, MemBasis, TopMetric};
use crate::views::dockerscope::helpers::{shorten_id, state_class_for};

/// One ranked container of the Top consumers board — everything the row needs,
/// already flattened out of `MetricsByVm` so the render pass does no lookups.
#[derive(Clone, PartialEq)]
pub struct TopConsumerRow {
    pub id: String,
    pub name: String,
    /// Source VM — `Some` in `/all` view, `None` when scoped to a single VM
    /// (there the VM is implicit and lives in the URL prefix).
    pub vm: Option<String>,
    pub image: String,
    pub state_class: &'static str,
    pub cpu: f64,
    pub mem_bytes: i64,
    /// Declared mem limit when set, otherwise the VM's host RAM — an unlimited
    /// container can claim the whole box (same rule as the container list).
    pub effective_mem_limit: Option<i64>,
    /// Whether `effective_mem_limit` came from `mem.limit` (true) or fell back
    /// to host RAM (false). Drives the tooltip wording.
    pub mem_limit_is_declared: bool,
    /// Writable-layer size in bytes — what the container wrote on top of its
    /// image, and the figure the disk board ranks on. `None` until the
    /// collector's slow size pass has reached the container.
    pub disk_bytes: Option<i64>,
    /// Size including the image layers. Tooltip only: containers share those
    /// layers, so ranking or summing on it would count the same bytes once per
    /// container.
    pub disk_root_fs: Option<i64>,
    /// The value this row was ranked by — CPU percent, memory bytes, memory
    /// percent-of-limit, or disk bytes, depending on the board and its basis.
    /// Whatever it is, it is the same number the row displays as its headline.
    pub value: f64,
}

impl TopConsumerRow {
    fn from(m: &MetricsByVm, metric: TopMetric, mem_basis: MemBasis) -> Self {
        let c = &m.container;
        let name = if c.names.is_empty() {
            shorten_id(&c.id, 12).to_string()
        } else {
            primary_name(&c.names).to_string()
        };
        // Ranking and the share percentages are arithmetic, so a non-finite value
        // has to be rejected at the boundary: a single NaN in the set makes `total`
        // NaN and every row's share unreadable. A collector built after the fix in
        // `get_cpu_usage` never sends one, but an older one in the fleet can.
        let cpu = c.cpu.usage.filter(|v| v.is_finite()).unwrap_or(0.0);
        let mem_bytes = c.mem.usage.unwrap_or(0);
        let (effective_mem_limit, mem_limit_is_declared) = match c.mem.limit {
            Some(v) if v > 0 => (Some(v), true),
            _ => (m.host_mem_total, false),
        };
        // Percent of what this container was allowed. Computed here rather than
        // through `mem_pct()` because the ranking needs it before the row exists.
        let reserved_pct = match effective_mem_limit {
            Some(limit) if limit > 0 => (mem_bytes as f64 / limit as f64) * 100.0,
            // Neither a declared limit nor a host RAM reading: there is nothing to
            // be a percentage of, so the row sorts to the bottom rather than
            // claiming a figure nobody measured.
            _ => 0.0,
        };
        let disk_bytes = c.disk.size_rw;

        Self {
            id: c.id.clone(),
            name,
            vm: m.vm.clone(),
            image: c.image.clone(),
            state_class: state_class_for(c.state.as_deref()),
            cpu,
            mem_bytes,
            effective_mem_limit,
            mem_limit_is_declared,
            disk_bytes,
            disk_root_fs: c.disk.size_root_fs,
            value: match metric {
                TopMetric::Cpu => cpu,
                TopMetric::Mem => match mem_basis {
                    MemBasis::Total => mem_bytes as f64,
                    MemBasis::Reserved => reserved_pct,
                },
                // A size nobody has measured yet ranks as zero, so the row sorts to
                // the bottom — but it keeps its `None`, and is shown as unknown
                // rather than as a container that wrote nothing.
                TopMetric::Disk => disk_bytes.unwrap_or(0) as f64,
            },
        }
    }

    pub fn mem_pct(&self) -> Option<f64> {
        let limit = self.effective_mem_limit?;
        if limit <= 0 {
            return None;
        }
        Some((self.mem_bytes as f64 / limit as f64) * 100.0)
    }

    /// Bar width, 0..100, scaled against the leader so the #1 row is always full.
    pub fn bar_pct(&self, max: f64) -> f64 {
        if max <= 0.0 {
            return 0.0;
        }
        ((self.value / max) * 100.0).clamp(0.0, 100.0)
    }

    /// Share of the whole ranked set, 0..100. `None` when nothing is consuming.
    pub fn share_pct(&self, total: f64) -> Option<f64> {
        if total <= 0.0 {
            return None;
        }
        Some((self.value / total) * 100.0)
    }
}

/// The ranked board plus the aggregates its header and bars are drawn from.
pub struct TopConsumers {
    pub rows: Vec<TopConsumerRow>,
    /// Sum of the metric over **every** container considered, not just the
    /// truncated Top-N — that's what the per-row share is measured against.
    pub total: f64,
    /// The leader's value; bars are scaled against it.
    pub max: f64,
    /// How many containers were ranked before the Top-N cut.
    pub ranked: usize,
    /// Total CPU percent across the ranked set (header summary).
    pub total_cpu: f64,
    /// Total memory bytes across the ranked set (header summary).
    pub total_mem: i64,
    /// Total writable-layer bytes across the ranked set (header summary). Covers
    /// only the containers that have been measured — see `disk_unmeasured`.
    pub total_disk: i64,
    /// Containers whose disk size the collector has not measured yet. They count
    /// towards `ranked` and add nothing to `total_disk`.
    pub disk_unmeasured: usize,
}

/// Rank the currently visible containers by `metric`, keeping the top `top_n`
/// (`0` keeps all). Input is the already filtered list, so the board follows
/// the search box and the state chips of the container column.
///
/// `mem_basis` only bites on the memory board; the CPU board ignores it, since
/// nothing in the payload says what a container was allowed of the CPU, and so
/// does the disk board, which has a single size to rank on.
pub fn build_top_consumers(
    containers: &[&MetricsByVm],
    metric: TopMetric,
    mem_basis: MemBasis,
    top_n: usize,
) -> TopConsumers {
    let mut rows: Vec<TopConsumerRow> = containers
        .iter()
        .map(|m| TopConsumerRow::from(*m, metric, mem_basis))
        .collect();

    let mut total = 0.0;
    let mut total_cpu = 0.0;
    let mut total_mem = 0i64;
    let mut total_disk = 0i64;
    let mut disk_unmeasured = 0usize;
    let mut max = 0.0f64;
    for row in rows.iter() {
        total += row.value;
        total_cpu += row.cpu;
        total_mem += row.mem_bytes;
        match row.disk_bytes {
            Some(bytes) => total_disk += bytes,
            None => disk_unmeasured += 1,
        }
        max = max.max(row.value);
    }
    let ranked = rows.len();

    // Descending by the NUMERIC metric — never by its rendered text, which would
    // put 9% above 10% and 900MB above 1GB. The name is a tiebreak only, so the
    // order doesn't jitter between polling ticks when several rows sit at 0.
    // `total_cmp` rather than `partial_cmp`: it is a total order over f64, so the
    // comparator stays consistent even if a non-finite value slipped through.
    rows.sort_by(|a, b| b.value.total_cmp(&a.value).then_with(|| a.name.cmp(&b.name)));

    if top_n > 0 && rows.len() > top_n {
        rows.truncate(top_n);
    }

    TopConsumers {
        rows,
        total,
        max,
        ranked,
        total_cpu,
        total_mem,
        total_disk,
        disk_unmeasured,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: i64 = 1024 * 1024;

    fn container(name: &str, size_rw: Option<i64>) -> MetricsByVm {
        serde_json::from_value(serde_json::json!({
            "vm": null,
            "url": "",
            "container": {
                "id": name,
                "image": "img",
                "names": [format!("/{}", name)],
                "enabled": true,
                "state": "running",
                "cpu": { "usage": 1.0 },
                "mem": { "usage": 10 },
                "disk": { "size_rw": size_rw, "size_root_fs": size_rw.map(|v| v + 100 * MIB) },
            },
        }))
        .unwrap()
    }

    fn disk_board(containers: &[MetricsByVm], top_n: usize) -> TopConsumers {
        let refs: Vec<&MetricsByVm> = containers.iter().collect();
        build_top_consumers(&refs, TopMetric::Disk, MemBasis::Total, top_n)
    }

    fn names(board: &TopConsumers) -> Vec<&str> {
        board.rows.iter().map(|row| row.name.as_str()).collect()
    }

    #[test]
    fn the_disk_board_ranks_by_writable_layer_bytes() {
        let containers = [
            container("small", Some(MIB)),
            container("big", Some(300 * MIB)),
            container("mid", Some(100 * MIB)),
        ];
        let board = disk_board(&containers, 0);

        assert_eq!(names(&board), vec!["big", "mid", "small"]);
        assert_eq!(board.total_disk, 401 * MIB);
        assert_eq!(board.rows[0].bar_pct(board.max), 100.0);
        // The image layers are carried for the tooltip, and stay out of the ranking.
        assert_eq!(board.rows[0].disk_root_fs, Some(400 * MIB));
        assert_eq!(board.rows[0].value, (300 * MIB) as f64);
    }

    #[test]
    fn an_unmeasured_container_sorts_last_and_is_not_passed_off_as_zero() {
        let containers = [
            container("pending", None),
            container("writer", Some(5 * MIB)),
        ];
        let board = disk_board(&containers, 0);

        assert_eq!(names(&board), vec!["writer", "pending"]);
        assert_eq!(board.rows[1].disk_bytes, None);
        assert_eq!(board.disk_unmeasured, 1);
        assert_eq!(board.total_disk, 5 * MIB);
        assert_eq!(board.ranked, 2);
    }

    #[test]
    fn the_cut_does_not_shrink_the_disk_total() {
        let containers = [
            container("a", Some(3 * MIB)),
            container("b", Some(2 * MIB)),
            container("c", Some(MIB)),
        ];
        let board = disk_board(&containers, 1);

        assert_eq!(names(&board), vec!["a"]);
        assert_eq!(board.total_disk, 6 * MIB);
        assert_eq!(board.rows[0].share_pct(board.total), Some(50.0));
    }
}
