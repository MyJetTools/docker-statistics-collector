use std::time::Duration;

use flurl::FlUrl;

use crate::app::AppContext;

pub enum RouteLogsResult {
    Ok(Vec<u8>),
    NotFound,
    PeerError(String),
}

pub async fn route_logs(app: &AppContext, container_id: &str, lines_number: u32) -> RouteLogsResult {
    if container_owned_locally(app, container_id).await {
        let bytes = docker_sdk::sdk::get_container_logs(
            app.settings_model.docker_url.as_str(),
            container_id,
            lines_number,
        )
        .await;
        return RouteLogsResult::Ok(bytes);
    }

    let Some((peer_url, _instance)) = app.peers_cache.find_peer_for_container(container_id).await
    else {
        return RouteLogsResult::NotFound;
    };

    let timeout = app.settings_model.peers_request_timeout();
    match fetch_logs_from_peer(&peer_url, container_id, lines_number, timeout).await {
        Ok(bytes) => RouteLogsResult::Ok(bytes),
        Err(err) => RouteLogsResult::PeerError(err),
    }
}

async fn container_owned_locally(app: &AppContext, container_id: &str) -> bool {
    app.cache
        .get_snapshot()
        .await
        .iter()
        .any(|c| c.id == container_id)
}

async fn fetch_logs_from_peer(
    peer_url: &str,
    container_id: &str,
    lines_number: u32,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let response = FlUrl::new(peer_url)
        .append_path_segment("api")
        .append_path_segment("containers")
        .append_path_segment("logs")
        .append_query_param("id", Some(container_id))
        .append_query_param("lines_number", Some(lines_number.to_string()))
        .set_timeout(timeout)
        .get()
        .await
        .map_err(|e| format!("peer {}: request failed: {:?}", peer_url, e))?;

    let status = response.get_status_code();
    let body = response
        .receive_body()
        .await
        .map_err(|e| format!("peer {}: receive_body failed: {:?}", peer_url, e))?;

    if status != 200 {
        return Err(format!(
            "peer {} returned status {} ({} bytes)",
            peer_url,
            status,
            body.len()
        ));
    }

    Ok(body)
}
