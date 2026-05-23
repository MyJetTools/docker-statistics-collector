use serde::{Deserialize, Serialize};

use crate::models::RequestError;

use super::{get_base_url, url_encode};

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

pub async fn get_processes(
    env: String,
    url: String,
    id: String,
) -> Result<Vec<ProcessHttpModel>, RequestError> {
    let endpoint = format!(
        "{}/api/processes?env={}&url={}&id={}",
        get_base_url(),
        url_encode(&env),
        url_encode(&url),
        url_encode(&id),
    );
    let resp = reqwest::Client::new().get(&endpoint).send().await?;
    Ok(resp.json::<Vec<ProcessHttpModel>>().await?)
}
