use std::time::Duration;

use flurl::IntoFlUrl;
use rust_extensions::date_time::DateTimeAsMicroseconds;
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
    /// RFC3339 timestamp of when the container's main process was last
    /// started. Updates on every `docker start` (so survives `docker create`
    /// but advances on restarts). `"0001-01-01T00:00:00Z"` when the container
    /// has never been started.
    #[serde(rename = "StartedAt", default)]
    pub started_at: Option<String>,
}

/// State of a container at inspect time. All fields are best-effort — `pid` is
/// `None` when the container isn't running, `started_at` is `None` when the
/// container has never been started or the timestamp can't be parsed.
#[derive(Debug, Clone, Copy, Default)]
pub struct ContainerStateInfo {
    pub pid: Option<u32>,
    /// Unix epoch seconds of the last container start. `None` when the
    /// container has never been started (sentinel `0001-01-01T...` timestamp)
    /// or the timestamp could not be parsed.
    pub started_at_unix_seconds: Option<i64>,
}

/// Single inspect call returning both the main PID and `StartedAt`. Callers
/// that only need one of those can drop the field; this avoids hitting the
/// daemon twice for `/proc` lookups + start-time tracking.
pub async fn get_container_state(url: String, container_id: String) -> Option<ContainerStateInfo> {
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

    let pid = if model.state.pid == 0 {
        None
    } else {
        Some(model.state.pid)
    };

    let started_at_unix_seconds = model.state.started_at.as_deref().and_then(parse_started_at);

    Some(ContainerStateInfo {
        pid,
        started_at_unix_seconds,
    })
}

/// Returns the host PID of a container's main process. Thin wrapper over
/// [`get_container_state`] — kept for the existing call sites in `proc_fd`.
pub async fn get_container_main_pid(url: String, container_id: String) -> Option<u32> {
    get_container_state(url, container_id).await?.pid
}

fn parse_started_at(raw: &str) -> Option<i64> {
    // Docker uses `"0001-01-01T00:00:00Z"` as a sentinel for "never started".
    if raw.starts_with("0001-") {
        return None;
    }
    let dt = DateTimeAsMicroseconds::parse_iso_string(raw)?;
    Some(dt.unix_microseconds / 1_000_000)
}
