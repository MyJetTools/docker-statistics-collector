use std::collections::BTreeMap;

use serde::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PortHttpModel {
    pub ip: Option<String>,
    #[serde(rename = "privatePort")]
    pub private_port: u16,
    #[serde(rename = "publicPort")]
    pub public_port: Option<u16>,
    #[serde(rename = "portType")]
    pub port_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VolumeHttpModel {
    #[serde(rename = "mountType")]
    pub mount_type: Option<String>,
    pub name: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub driver: Option<String>,
    pub mode: Option<String>,
    pub rw: Option<bool>,
    pub propagation: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ContainerModel {
    pub id: String,
    pub image: String,
    pub names: Vec<String>,
    pub labels: Option<BTreeMap<String, String>>,
    pub enabled: bool,
    pub created: Option<i64>,
    pub status: Option<String>,
    pub state: Option<String>,
    #[serde(default)]
    pub instance: String,
    pub cpu: CpuUsageJsonMode,
    pub mem: MemUsageJsonMode,
    #[serde(default)]
    pub files: FilesUsageJsonMode,
    pub cpu_usage_history: Option<Vec<f64>>,
    pub mem_usage_history: Option<Vec<i64>>,
    pub open_files_history: Option<Vec<i64>>,

    pub ports: Option<Vec<PortHttpModel>>,
    #[serde(default)]
    pub volumes: Option<Vec<VolumeHttpModel>>,
}

impl ContainerModel {
    pub fn filter_me(&self, value: &str) -> bool {
        if value == "" {
            return true;
        }

        if self.id.contains(value) {
            return true;
        }

        let value = value.to_lowercase();

        if self.image.to_lowercase().contains(&value) {
            return true;
        }

        for name in &self.names {
            if name.to_lowercase().contains(&value) {
                return true;
            }
        }

        if let Some(labels) = &self.labels {
            for (key, v) in labels {
                if key.to_lowercase().contains(&value) {
                    return true;
                }

                if v.to_lowercase().contains(&value) {
                    return true;
                }
            }
        }

        return false;
    }

    #[cfg(feature = "server")]
    pub fn update(&mut self, src: ContainerJsonModel) {
        self.cpu = src.cpu;
        self.mem = src.mem;
        self.files = src.files;
        self.labels = src.labels;
        self.enabled = src.enabled;
        self.image = src.image;
        self.instance = src.instance;
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ContainerJsonModel {
    pub id: String,
    pub image: String,
    pub names: Vec<String>,
    pub labels: Option<BTreeMap<String, String>>,
    pub enabled: bool,
    pub created: Option<i64>,
    pub state: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub instance: String,
    pub cpu: CpuUsageJsonMode,
    pub mem: MemUsageJsonMode,
    #[serde(default)]
    pub files: FilesUsageJsonMode,
    pub ports: Option<Vec<PortHttpModel>>,
    #[serde(default)]
    pub volumes: Option<Vec<VolumeHttpModel>>,
}

#[derive(Serialize, Deserialize)]
pub struct StatisticsContract {
    pub vm: String,
    pub containers: Vec<ContainerJsonModel>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CpuUsageJsonMode {
    pub usage: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemUsageJsonMode {
    pub usage: Option<i64>,
    pub available: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FilesUsageJsonMode {
    /// File descriptors currently open by the container's main process.
    pub open: Option<i64>,
    /// `nofile` soft limit (`RLIMIT_NOFILE`) of the container's main process.
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MetricsByVm {
    pub vm: Option<String>,
    pub url: String,
    pub container: ContainerModel,
}
