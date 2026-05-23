use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHttpModel {
    pub pid: u32,
    pub cmd: String,
    pub open_files: Option<i64>,
    pub fd_limit: Option<i64>,
}

#[get("/api/processes?env&url&id")]
pub async fn get_processes(
    env: String,
    url: String,
    id: String,
) -> Result<Vec<ProcessHttpModel>, ServerFnError> {
    let fl_url = crate::server::APP_CTX
        .get_fl_url(env.as_str(), url.as_str())
        .await;

    match crate::server::http_client::get_processes(fl_url, id).await {
        Ok(result) => Ok(result),
        Err(err) => Err(ServerFnError::new(err)),
    }
}
