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

pub async fn get_list_of_containers(url: String) -> Vec<ContainerJsonModel> {
    let mut result = url
        .as_str()
        .append_path_segment("containers")
        .append_path_segment("json")
        .append_query_param("all", Some("true"))
        .set_timeout(Duration::from_secs(3))
        .get()
        .await
        .unwrap();

    let body = if result.get_status_code() != 200 {
        println!("url: {}", url);
        println!("Status code: {}", result.get_status_code());
        println!("Headers: {:#?}", result.get_headers());
        let body = result.get_body_as_slice().await.unwrap();

        println!("Body Len: {}", body.len());
        body
    } else {
        let body = result.get_body_as_slice().await.unwrap();
        body
    };

    serde_json::from_slice(body).unwrap()
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
