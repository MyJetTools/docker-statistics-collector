use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::{MetricsByVm, RequestError, VmModel};

use super::{get_base_url, url_encode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestApiModel {
    pub vms: BTreeMap<String, VmModel>,
    pub metrics: Option<Vec<MetricsByVm>>,
}

pub async fn get_vm_cpu_and_mem(
    env: String,
    selected_vm: String,
) -> Result<RequestApiModel, RequestError> {
    let url = format!(
        "{}/api/vm_cpu_and_mem?env={}&selected_vm={}",
        get_base_url(),
        url_encode(&env),
        url_encode(&selected_vm),
    );
    let resp = reqwest::Client::new().get(&url).send().await?;
    Ok(resp.json::<RequestApiModel>().await?)
}
