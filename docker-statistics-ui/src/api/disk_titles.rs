use flurl::{body::HttpRequestBody, FlUrl};
use serde::{Deserialize, Serialize};

use crate::models::RequestError;

use super::handle_http_response;

#[derive(Serialize)]
struct SetDiskTitleRequest<'s> {
    env: &'s str,
    vm: &'s str,
    disk: &'s str,
    title: &'s str,
}

#[derive(Deserialize)]
struct SetDiskTitleResponse {
    title: Option<String>,
}

/// Name a host disk; an empty `title` removes the name. `disk` is its mount
/// point. Returns the title as the api stored it — trimmed, `None` once removed.
pub async fn set_disk_title(
    env: &str,
    vm: &str,
    disk: &str,
    title: &str,
) -> Result<Option<String>, RequestError> {
    let request = SetDiskTitleRequest {
        env,
        vm,
        disk,
        title,
    };

    let response = FlUrl::new("/api/disk-title")
        .post(HttpRequestBody::as_json(&request))
        .await;

    let response: SetDiskTitleResponse = handle_http_response(response).await?;
    Ok(response.title)
}
