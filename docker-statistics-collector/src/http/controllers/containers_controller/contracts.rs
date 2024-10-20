use std::collections::HashMap;

use my_http_server::macros::MyHttpObjectStructure;
use serde::Serialize;

use crate::app::ServiceInfo;

#[derive(MyHttpObjectStructure, Serialize)]
pub struct ContainersHttpResponse {
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
    pub created: i64,
    pub cpu: CpuUsageJsonMode,
    pub mem: MemUsageJsonMode,
    pub ports: Vec<PortHttpModel>,
}

impl ContainerJsonModel {
    pub fn new(itm: ServiceInfo) -> Self {
        Self {
            id: itm.id,
            image: itm.image,
            enabled: itm.running,
            cpu: CpuUsageJsonMode {
                usage: itm.cpu_usage,
            },
            mem: MemUsageJsonMode {
                usage: itm.mem_usage,
                available: itm.mem_available,
                limit: itm.mem_limit,
            },
            names: itm.names,
            labels: itm.labels,
            created: itm.created,

            ports: itm
                .ports
                .into_iter()
                .map(|itm| PortHttpModel {
                    ip: itm.ip,
                    private_port: itm.private_port,
                    public_port: itm.public_port,
                    port_type: itm.port_type,
                })
                .collect(),
        }
    }
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

#[derive(Serialize, MyHttpObjectStructure)]
pub struct PortHttpModel {
    pub ip: Option<String>,
    #[serde(rename = "privatePort")]
    pub private_port: u16,
    #[serde(rename = "publicPort")]
    pub public_port: Option<u16>,
    #[serde(rename = "portType")]
    pub port_type: String,
}
