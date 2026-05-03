use serde::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmModel {
    pub api_url: String,
    pub cpu: f64,
    pub mem: i64,
    pub mem_limit: i64,
    pub containers_amount: usize,
}
