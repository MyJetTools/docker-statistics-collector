use std::time::Duration;

use flurl::IntoFlUrl;
use serde::*;

/// Subset of the Docker Engine `GET /containers/{id}/json` (inspect) response.
#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerInspectJsonModel {
    #[serde(rename = "State")]
    pub state: ContainerStateJsonModel,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerStateJsonModel {
    /// Host PID of the container's main process (PID 1 inside the container —
    /// the process started by the image `ENTRYPOINT`/`CMD`). `0` when the
    /// container is not running.
    #[serde(rename = "Pid")]
    pub pid: u32,
}

/// Returns the host PID of a container's main process.
///
/// This is exactly the process started at container startup. Returns `None`
/// when the container is not running, the request fails, or the response
/// cannot be parsed.
pub async fn get_container_main_pid(url: String, container_id: String) -> Option<u32> {
    let mut response = url
        .as_str()
        .with_header("host", "localhost")
        .append_path_segment("containers")
        .append_path_segment(container_id)
        .append_path_segment("json")
        .set_timeout(Duration::from_secs(5))
        .get()
        .await
        .ok()?;

    if response.get_status_code() != 200 {
        return None;
    }

    let body = response.get_body_as_slice().await.ok()?;
    let model: ContainerInspectJsonModel = serde_json::from_slice(body).ok()?;

    if model.state.pid == 0 {
        return None;
    }

    Some(model.state.pid)
}
