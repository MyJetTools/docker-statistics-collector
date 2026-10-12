use flurl::FlUrl;

pub async fn get_logs(fl_url: FlUrl, container_id: String, lines: u32) -> Result<Vec<u8>, String> {
    let url_response = fl_url
        .append_path_segment("api")
        .append_path_segment("containers")
        .append_path_segment("logs")
        .append_query_param("id", Some(container_id))
        .append_query_param("lines_number", Some(lines.to_string()))
        .get()
        .await;

    let mut url_response = match url_response {
        Ok(url_response) => url_response,
        Err(err) => return Err(format!("Error: {:?}", err)),
    };

    super::read_body(&mut url_response)
        .await
        .map_err(|err| format!("Error: {:?}", err))
}
