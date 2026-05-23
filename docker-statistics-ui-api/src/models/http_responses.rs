use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{MetricsByVm, VmModel};

#[derive(Serialize, Deserialize)]
pub struct EnvsHttpModel {
    pub envs: Vec<String>,
    pub request_pass_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestApiModel {
    pub vms: BTreeMap<String, VmModel>,
    pub metrics: Option<Vec<MetricsByVm>>,
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
