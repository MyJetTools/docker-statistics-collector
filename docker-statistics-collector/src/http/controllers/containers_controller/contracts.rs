use std::collections::HashMap;

use my_http_server::macros::MyHttpObjectStructure;
use serde::Serialize;

#[derive(MyHttpObjectStructure, Serialize)]
pub struct ContainersHtpResponse {
    pub vm: String,
    pub containers: Vec<ContainerJsonModel>,
}

#[derive(Serialize, MyHttpObjectStructure)]
pub struct ContainerJsonModel {
    pub id: String,
    pub image: String,
    pub names: Vec<String>,
    pub labels: Option<HashMap<String, String>>,
    pub enabled: bool,
    pub cpu: CpuUsageJsonMode,
    pub mem: MemUsageJsonMode,
}
#[derive(Serialize, MyHttpObjectStructure)]
pub struct CpuUsageJsonMode {
    pub usage: Option<f64>,
}

#[derive(Serialize, MyHttpObjectStructure)]
pub struct MemUsageJsonMode {
    pub usage: Option<i64>,
    pub available: Option<i64>,
    pub limit: Option<i64>,
}
