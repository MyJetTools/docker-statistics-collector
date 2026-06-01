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
        let result = self.state == "running";

        if !result {
            eprintln!(
                "Container {} is not running. State is: {}",
                self.image, self.state
            );
        }

        result
    }
}

pub async fn get_list_of_containers(url: String) -> Vec<ContainerJsonModel> {
    let mut result = url
        .as_str()
        .with_header("host", "localhost")
        .append_path_segment("containers")
        .append_path_segment("json")
        .append_query_param("all", Some("true"))
        .set_timeout(Duration::from_secs(5))
        .do_not_reuse_connection()
        .get()
        .await
        .unwrap();

    let status_code = result.get_status_code();

    if status_code != 200 {
        println!("url: {}", url);
        println!("Status code: {}", status_code);
        println!("Headers: {:#?}", result.get_headers());
        let body = result.get_body_as_slice().await.unwrap();
        println!("Body Len: {}", body.len());
        println!("Body: {:?}", std::str::from_utf8(body));
        println!("BodyAsBytes: {:?}", body);
        panic!("Docker returned non-200 status: {}", status_code);
    }

    let body = result.get_body_as_slice().await.unwrap();
    serde_json::from_slice(body).unwrap()
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
