use serde::{Deserialize, Serialize};

use crate::models::RequestError;

use super::{get_base_url, url_encode};

/// Exec-permission window of one VM, as reported by the api.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecPermissionModel {
    pub instance: String,
    pub enabled: bool,
    pub seconds_left: i64,
}

fn build_endpoint(action: &str, env: &str, url: &str, instance: &str) -> String {
    format!(
        "{}/api/exec-permission{}?env={}&url={}&instance={}",
        get_base_url(),
        action,
        url_encode(env),
        url_encode(url),
        url_encode(instance),
    )
}

pub async fn get_exec_permission(
    env: String,
    url: String,
    instance: String,
) -> Result<ExecPermissionModel, RequestError> {
    let endpoint = build_endpoint("", &env, &url, &instance);
    let resp = reqwest::Client::new().get(&endpoint).send().await?;
    Ok(resp.json::<ExecPermissionModel>().await?)
}

pub async fn enable_exec_permission(
    env: String,
    url: String,
    instance: String,
) -> Result<ExecPermissionModel, RequestError> {
    let endpoint = build_endpoint("/enable", &env, &url, &instance);
    let resp = reqwest::Client::new().post(&endpoint).send().await?;
    Ok(resp.json::<ExecPermissionModel>().await?)
}

pub async fn disable_exec_permission(
    env: String,
    url: String,
    instance: String,
) -> Result<ExecPermissionModel, RequestError> {
    let endpoint = build_endpoint("/disable", &env, &url, &instance);
    let resp = reqwest::Client::new().post(&endpoint).send().await?;
    Ok(resp.json::<ExecPermissionModel>().await?)
}
