use std::time::Duration;

use flurl::IntoFlUrl;
use serde::*;

/// Response of the Docker Engine `GET /containers/{id}/top` endpoint.
#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerTopJsonModel {
    #[serde(rename = "Titles")]
    pub titles: Vec<String>,
    #[serde(rename = "Processes")]
    pub processes: Vec<Vec<String>>,
}

/// A single process running inside a container, as reported by `docker top`.
#[derive(Debug, Clone)]
pub struct ContainerProcess {
    /// Host PID — usable to read `/proc/<pid>` once the host `/proc` is visible.
    pub pid: u32,
    /// Command line of the process.
    pub cmd: String,
}

/// Returns every process running inside a container with its host PID and
/// command line. Returns `None` when the container is not running, the request
/// fails, or the `PID` column cannot be located.
pub async fn get_container_processes(
    url: String,
    container_id: String,
) -> Option<Vec<ContainerProcess>> {
    let mut response = url
        .as_str()
        .with_header("host", "localhost")
        .append_path_segment("containers")
        .append_path_segment(container_id)
        .append_path_segment("top")
        .set_timeout(Duration::from_secs(5))
        .get()
        .await
        .ok()?;

    if response.get_status_code() != 200 {
        return None;
    }

    let body = response.get_body_as_slice().await.ok()?;
    let model: ContainerTopJsonModel = serde_json::from_slice(body).ok()?;

    let pid_column = model.titles.iter().position(|title| title == "PID")?;
    let cmd_column = model
        .titles
        .iter()
        .position(|title| title == "CMD" || title == "COMMAND");

    let mut result = Vec::new();
    for process in &model.processes {
        let Some(pid) = process
            .get(pid_column)
            .and_then(|pid| pid.trim().parse::<u32>().ok())
        else {
            continue;
        };

        let cmd = cmd_column
            .and_then(|index| process.get(index))
            .cloned()
            .unwrap_or_default();

        result.push(ContainerProcess { pid, cmd });
    }

    Some(result)
}
