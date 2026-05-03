use crate::app::AppContext;

pub enum RouteLogsResult {
    Ok(Vec<u8>),
    NotFound,
    PeerError(String),
}

pub async fn route_logs(app: &AppContext, container_id: &str, lines_number: u32) -> RouteLogsResult {
    crate::peers_client::fanout_logs(app, container_id, lines_number).await
}
