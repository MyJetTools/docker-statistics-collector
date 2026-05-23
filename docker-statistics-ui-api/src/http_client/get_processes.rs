use flurl::FlUrl;
use serde::Deserialize;

use crate::models::ProcessHttpModel;

#[derive(Deserialize)]
struct ProcessesResponseModel {
    processes: Vec<ProcessHttpModel>,
}

pub async fn get_processes(
    fl_url: FlUrl,
    container_id: String,
) -> Result<Vec<ProcessHttpModel>, String> {
    let response = fl_url
        .append_path_segment("api")
        .append_path_segment("containers")
        .append_path_segment("processes")
        .append_query_param("id", Some(container_id))
        .get()
        .await;

    let mut response = match response {
        Ok(response) => response,
        Err(err) => return Err(format!("Error: {:?}", err)),
    };

    let status_code = response.get_status_code();

    let body = match response.get_body_as_slice().await {
        Ok(body) => body,
        Err(err) => return Err(format!("Error reading body: {:?}", err)),
    };

    if status_code != 200 {
        return Err(format!("Collector returned status {}", status_code));
    }

    match serde_json::from_slice::<ProcessesResponseModel>(body) {
        Ok(parsed) => Ok(parsed.processes),
        Err(err) => Err(format!("Parse error: {}", err)),
    }
}
