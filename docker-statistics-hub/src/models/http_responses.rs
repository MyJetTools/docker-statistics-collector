use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{MetricsByVm, VmModel};

#[derive(Serialize, Deserialize)]
pub struct EnvsHttpModel {
    pub envs: Vec<String>,
    /// Identity from the `x-ssl-user` header, empty when the reverse proxy
    /// didn't inject one. UI shows this so the operator knows under whose
    /// principal the page is actually scoped.
    #[serde(default)]
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestApiModel {
    pub vms: BTreeMap<String, VmModel>,
    pub metrics: Option<Vec<MetricsByVm>>,
    /// Seconds since this env last produced a usable answer from its master collector.
    /// `None` before the first successful poll.
    ///
    /// Every reading is now a live Docker scan that can time out at three layers
    /// (api → master, master → peer, collector → daemon), and a failed poll leaves the
    /// previous snapshot in place. Without this the UI cannot tell a quiet fleet from a
    /// frozen one — which is exactly the distinction that matters during an incident.
    #[serde(default)]
    pub data_age_secs: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogLineHttpModel {
    pub tp: u8,
    pub line: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessHttpModel {
    pub pid: u32,
    pub cmd: String,
    pub open_files: Option<i64>,
    pub fd_limit: Option<i64>,
    pub mem_rss: Option<i64>,
    pub mem_vsize: Option<i64>,
    pub threads: Option<i64>,
}
