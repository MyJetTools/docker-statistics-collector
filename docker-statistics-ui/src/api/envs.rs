use serde::{Deserialize, Serialize};

use crate::models::RequestError;

use super::get_base_url;

#[derive(Serialize, Deserialize)]
pub struct EnvsHttpModel {
    pub envs: Vec<String>,
    pub request_pass_key: bool,
}

pub async fn get_envs() -> Result<EnvsHttpModel, RequestError> {
    let url = format!("{}/api/envs", get_base_url());
    let resp = reqwest::Client::new().get(&url).send().await?;
    Ok(resp.json::<EnvsHttpModel>().await?)
}
