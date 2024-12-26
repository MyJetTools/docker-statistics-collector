use std::collections::HashMap;

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
}

impl ContainerJsonModel {
    pub fn created_as_date_time(&self) -> DateTimeAsMicroseconds {
        self.created.into()
    }

    pub fn is_running(&self) -> bool {
        let result = self.state == "running";

        if !result {
            println!(
                "Container {} is not running. State is: {}",
                self.image, self.state
            );
        }

        result
    }
}

pub async fn get_list_of_containers(url: String, api_version: &str) -> Vec<ContainerJsonModel> {
    let mut result = url
        .append_path_segment(api_version)
        .append_path_segment("containers")
        .append_path_segment("json")
        .append_query_param("all", Some("true"))
        .with_header("Host", "docker")
        .with_header("Accept", "*/*")
        .with_header("User-Agent", "Rust application")
        .get()
        .await
        .unwrap();

    println!("Status code: {}", result.get_status_code());
    println!("Headers: {:#?}", result.get_headers());
    let body = result.body_as_str().await.unwrap();

    println!("Body: {:?}", body);

    serde_json::from_str(body).unwrap()
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
