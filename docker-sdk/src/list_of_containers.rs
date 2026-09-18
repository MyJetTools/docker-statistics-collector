use std::{collections::HashMap, time::Duration};

use flurl::IntoFlUrl;
use rust_extensions::date_time::DateTimeAsMicroseconds;
use serde::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerJsonModel {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Names")]
    pub names: Vec<String>,
    #[serde(rename = "Image")]
    pub image: String,
    #[serde(rename = "Labels")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(rename = "Created")]
    pub created: i64,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "Status")]
    pub status: String,

    #[serde(rename = "Ports")]
    pub ports: Option<Vec<ContainerStatsPortModel>>,

    #[serde(rename = "Mounts")]
    pub mounts: Option<Vec<ContainerMountModel>>,

    /// Writable-layer size in bytes — only present when the list is fetched
    /// with `size=true` (expensive: Docker walks the storage layers).
    #[serde(rename = "SizeRw", default)]
    pub size_rw: Option<i64>,
    /// Total size in bytes including the read-only image layers — only present
    /// with `size=true`.
    #[serde(rename = "SizeRootFs", default)]
    pub size_root_fs: Option<i64>,
}

impl ContainerJsonModel {
    pub fn created_as_date_time(&self) -> DateTimeAsMicroseconds {
        self.created.into()
    }

    pub fn is_running(&self) -> bool {
        // No logging here: this is called twice per container on every live scan,
        // which is now the HTTP request path. A fleet with dozens of stopped
        // containers would bury the peer/daemon errors that stderr is actually for.
        self.state == "running"
    }
}

/// Lists containers. `Err` carries a human-readable reason.
///
/// This runs on the HTTP request path now that the collector is stateless, so it
/// must never panic: a daemon hiccup has to surface as a 500 on one request, not
/// take the process (or the whole env's cached view) with it.
pub async fn get_list_of_containers(
    url: String,
) -> Result<Vec<ContainerJsonModel>, String> {
    let mut result = url
        .as_str()
        .with_header("host", "localhost")
        .append_path_segment("containers")
        .append_path_segment("json")
        .append_query_param("all", Some("true"))
        .set_timeout(Duration::from_secs(5))
        // `set_timeout` bounds request→headers only; fl-url leaves the body read
        // unbounded by default, so a daemon that answers 200 and then stalls
        // mid-body would hang this call forever.
        .set_response_body_timeout(Duration::from_secs(5))
        .do_not_reuse_connection()
        .get()
        .await
        .map_err(|err| format!("docker {}: list request failed: {:?}", url, err))?;

    let status_code = result.get_status_code();

    if status_code != 200 {
        return Err(format!(
            "docker {}: list returned status {}",
            url, status_code
        ));
    }

    let body = result
        .get_body_as_slice()
        .await
        .map_err(|err| format!("docker {}: list body read failed: {:?}", url, err))?;

    serde_json::from_slice(body)
        .map_err(|err| format!("docker {}: list parse failed: {}", url, err))
}

#[derive(Deserialize)]
struct ContainerSizeInspect {
    #[serde(rename = "SizeRw", default)]
    size_rw: Option<i64>,
    #[serde(rename = "SizeRootFs", default)]
    size_root_fs: Option<i64>,
}

/// Disk usage for a SINGLE container, via `GET /containers/{id}/json?size=true`.
/// This is EXPENSIVE (Docker walks that container's storage layers), so callers
/// compute one container per tick rather than the whole batch at once. Returns
/// `(size_rw, size_root_fs)` in bytes; `(None, None)` on any failure.
pub async fn get_container_size(url: String, container_id: &str) -> (Option<i64>, Option<i64>) {
    let result = url
        .as_str()
        .with_header("host", "localhost")
        .append_path_segment("containers")
        .append_path_segment(container_id)
        .append_path_segment("json")
        .append_query_param("size", Some("true"))
        .set_timeout(Duration::from_secs(30))
        .set_response_body_timeout(Duration::from_secs(30))
        .do_not_reuse_connection()
        .get()
        .await;

    let mut result = match result {
        Ok(r) => r,
        Err(err) => {
            eprintln!("get_container_size {}: request failed: {:?}", container_id, err);
            return (None, None);
        }
    };

    if result.get_status_code() != 200 {
        eprintln!(
            "get_container_size {}: docker returned status {}",
            container_id,
            result.get_status_code()
        );
        return (None, None);
    }

    let body = match result.get_body_as_slice().await {
        Ok(b) => b,
        Err(err) => {
            eprintln!("get_container_size {}: body read failed: {:?}", container_id, err);
            return (None, None);
        }
    };

    match serde_json::from_slice::<ContainerSizeInspect>(body) {
        Ok(p) => (p.size_rw, p.size_root_fs),
        Err(err) => {
            eprintln!("get_container_size {}: parse failed: {}", container_id, err);
            (None, None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerMountModel {
    #[serde(rename = "Type")]
    pub mount_type: Option<String>,
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "Source")]
    pub source: Option<String>,
    #[serde(rename = "Destination")]
    pub destination: Option<String>,
    #[serde(rename = "Driver")]
    pub driver: Option<String>,
    #[serde(rename = "Mode")]
    pub mode: Option<String>,
    #[serde(rename = "RW")]
    pub rw: Option<bool>,
    #[serde(rename = "Propagation")]
    pub propagation: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerStatsPortModel {
    #[serde(rename = "IP")]
    pub ip: Option<String>,
    #[serde(rename = "PrivatePort")]
    pub private_port: u16,
    #[serde(rename = "PublicPort")]
    pub public_port: Option<u16>,
    #[serde(rename = "Type")]
    pub r#type: String,
}
