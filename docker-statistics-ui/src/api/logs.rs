use serde::{Deserialize, Serialize};

use crate::models::RequestError;

use super::{get_base_url, url_encode};

#[derive(Debug, Serialize, Deserialize)]
pub struct LogLineHttpModel {
    pub tp: u8,
    pub line: String,
}

pub async fn get_logs(
    env: String,
    url: String,
    id: String,
    lines_amount: u32,
) -> Result<Vec<LogLineHttpModel>, RequestError> {
    let endpoint = format!(
        "{}/api/logs?env={}&url={}&id={}&lines_amount={}",
        get_base_url(),
        url_encode(&env),
        url_encode(&url),
        url_encode(&id),
        lines_amount,
    );
    let resp = reqwest::Client::new().get(&endpoint).send().await?;
    Ok(resp.json::<Vec<LogLineHttpModel>>().await?)
}
