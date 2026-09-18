use crate::models::MetricsByVm;
use crate::states::{primary_name, TopMetric};
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
    /// The value this row was ranked by: CPU percent, or memory bytes as f64.
    pub value: f64,
}

impl TopConsumerRow {
    fn from(m: &MetricsByVm, metric: TopMetric) -> Self {
        let c = &m.container;
        let name = if c.names.is_empty() {
            shorten_id(&c.id, 12).to_string()
        } else {
            primary_name(&c.names).to_string()
        };
        let cpu = c.cpu.usage.unwrap_or(0.0);
        let mem_bytes = c.mem.usage.unwrap_or(0);
        let (effective_mem_limit, mem_limit_is_declared) = match c.mem.limit {
            Some(v) if v > 0 => (Some(v), true),
            _ => (m.host_mem_total, false),
        };
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
            value: match metric {
                TopMetric::Cpu => cpu,
                TopMetric::Mem => mem_bytes as f64,
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
}

/// Rank the currently visible containers by `metric`, keeping the top `top_n`
/// (`0` keeps all). Input is the already filtered list, so the board follows
/// the search box and the state chips of the container column.
pub fn build_top_consumers(
    containers: &[&MetricsByVm],
    metric: TopMetric,
    top_n: usize,
) -> TopConsumers {
    let mut rows: Vec<TopConsumerRow> = containers
        .iter()
        .map(|m| TopConsumerRow::from(*m, metric))
        .collect();

    let mut total = 0.0;
    let mut total_cpu = 0.0;
    let mut total_mem = 0i64;
    let mut max = 0.0f64;
    for row in rows.iter() {
        total += row.value;
        total_cpu += row.cpu;
        total_mem += row.mem_bytes;
        max = max.max(row.value);
    }
    let ranked = rows.len();

    // Descending by metric; ties fall back to the name so the order doesn't
    // jitter between polling ticks when several rows sit at 0.
    rows.sort_by(|a, b| {
        b.value
            .partial_cmp(&a.value)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });

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
    }
}
