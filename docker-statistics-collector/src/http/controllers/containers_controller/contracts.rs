use std::collections::HashMap;

use my_http_server::macros::MyHttpObjectStructure;
use serde::{Deserialize, Serialize};

use crate::app::ServiceInfo;

#[derive(MyHttpObjectStructure, Serialize, Deserialize)]
pub struct ContainersHttpResponse {
    pub vm: String,
    pub containers: Vec<ContainerJsonModel>,
}

#[derive(Serialize, Deserialize, MyHttpObjectStructure)]
pub struct ContainerJsonModel {
    pub id: String,
    pub image: String,
    pub names: Vec<String>,
    pub labels: Option<HashMap<String, String>>,
    pub enabled: bool,
    pub created: i64,
    pub state: String,
    pub status: String,
    pub instance: String,
    pub cpu: CpuUsageJsonMode,
    pub mem: MemUsageJsonMode,
    pub ports: Vec<PortHttpModel>,
}

impl ContainerJsonModel {
    pub fn new(itm: ServiceInfo, instance: String) -> Self {
        Self {
            id: itm.id,
            image: itm.image,
            enabled: itm.running,
            instance,
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

            state: itm.state,
            status: itm.status,

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

    pub fn into_service_info(self) -> ServiceInfo {
        use crate::app::ServiceInfoPortModel;
        ServiceInfo {
            id: self.id,
            image: self.image,
            names: self.names,
            labels: self.labels,
            running: self.enabled,
            created: self.created,
            state: self.state,
            status: self.status,
            mem_available: self.mem.available,
            mem_limit: self.mem.limit,
            mem_usage: self.mem.usage,
            cpu_usage: self.cpu.usage,
            ports: self
                .ports
                .into_iter()
                .map(|p| ServiceInfoPortModel {
                    ip: p.ip,
                    private_port: p.private_port,
                    public_port: p.public_port,
                    port_type: p.port_type,
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, MyHttpObjectStructure)]
pub struct CpuUsageJsonMode {
    pub usage: Option<f64>,
}

#[derive(Serialize, Deserialize, MyHttpObjectStructure)]
pub struct MemUsageJsonMode {
    pub usage: Option<i64>,
    pub available: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, MyHttpObjectStructure)]
pub struct PortHttpModel {
    pub ip: Option<String>,
    #[serde(rename = "privatePort")]
    pub private_port: u16,
    #[serde(rename = "publicPort")]
    pub public_port: Option<u16>,
    #[serde(rename = "portType")]
    pub port_type: String,
}
