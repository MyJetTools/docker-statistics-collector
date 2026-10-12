use std::rc::Rc;

/// A metric the hub keeps the history of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryMetric {
    Cpu,
    Mem,
    Files,
    NetIn,
    NetOut,
}

impl HistoryMetric {
    pub const ALL: [HistoryMetric; 5] = [
        HistoryMetric::Cpu,
        HistoryMetric::Mem,
        HistoryMetric::Files,
        HistoryMetric::NetIn,
        HistoryMetric::NetOut,
    ];

    /// Its name in the hub's api — and the modifier class that colours its chart.
    /// A CLASS, never an inline `var(--…)`: see `TopMetric::fill_class`.
    pub fn as_key(&self) -> &'static str {
        match self {
            HistoryMetric::Cpu => "cpu",
            HistoryMetric::Mem => "mem",
            HistoryMetric::Files => "files",
            HistoryMetric::NetIn => "net-in",
            HistoryMetric::NetOut => "net-out",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            HistoryMetric::Cpu => "CPU",
            HistoryMetric::Mem => "Memory",
            HistoryMetric::Files => "Open files",
            HistoryMetric::NetIn => "Net in",
            HistoryMetric::NetOut => "Net out",
        }
    }
}

/// How far back a history chart looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryPeriod {
    Hour,
    SixHours,
    Day,
    ThreeDays,
    Week,
}

impl HistoryPeriod {
    pub const ALL: [HistoryPeriod; 5] = [
        HistoryPeriod::Hour,
        HistoryPeriod::SixHours,
        HistoryPeriod::Day,
        HistoryPeriod::ThreeDays,
        HistoryPeriod::Week,
    ];

    pub fn seconds(&self) -> i64 {
        match self {
            HistoryPeriod::Hour => 3600,
            HistoryPeriod::SixHours => 6 * 3600,
            HistoryPeriod::Day => 24 * 3600,
            HistoryPeriod::ThreeDays => 3 * 24 * 3600,
            HistoryPeriod::Week => 7 * 24 * 3600,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            HistoryPeriod::Hour => "1h",
            HistoryPeriod::SixHours => "6h",
            HistoryPeriod::Day => "24h",
            HistoryPeriod::ThreeDays => "3d",
            HistoryPeriod::Week => "7d",
        }
    }
}

/// What a history dialog is about, and the metric it opens on: a service — a
/// container by its name on its host, which is what the hub keeps history by — or,
/// with no service, the machine as a whole: what its containers come to.
#[derive(Clone, PartialEq)]
pub struct ServiceHistoryModel {
    pub env: Rc<String>,
    pub vm: String,
    pub service: Option<String>,
    pub metric: HistoryMetric,
}

impl ServiceHistoryModel {
    /// The header of the dialog: whose history it is.
    pub fn title(&self) -> String {
        match self.service.as_deref() {
            Some(service) => format!("History · {} · {}", service, self.vm),
            None => format!("History · {}", self.vm),
        }
    }
}
