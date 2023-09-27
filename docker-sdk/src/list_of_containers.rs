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
}

impl ContainerJsonModel {
    pub fn created_as_date_time(&self) -> DateTimeAsMicroseconds {
        self.created.into()
    }

    pub fn is_running(&self) -> bool {
        self.state == "running"
    }
}

pub async fn get_list_of_containers(url: String) -> Vec<ContainerJsonModel> {
    url.append_path_segment("containers")
        .append_path_segment("json")
        .append_query_param("all", Some("true"))
        .get()
        .await
        .unwrap()
        .get_json()
        .await
        .unwrap()
}
