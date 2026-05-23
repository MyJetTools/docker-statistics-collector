use std::time::Duration;

use flurl::FlUrl;

use crate::app::{AppContext, ServiceInfo};
use crate::http::controllers::containers_controller::contracts::{
    ContainerProcessesHttpResponse, ContainersHttpResponse, HostMemEntryHttpModel, ProcessHttpModel,
};
use crate::http::controllers::containers_controller::RouteLogsResult;

/// Result of fanning out `/api/containers/local` to every configured peer.
/// Each tuple is `(peer_instance_name, peer_containers_as_serviceinfo, peer_host_mem_entries)`.
/// Peers that failed are logged to stderr and skipped (best-effort merge).
pub async fn fanout_local_containers(
    app: &AppContext,
) -> Vec<(String, Vec<ServiceInfo>, Vec<HostMemEntryHttpModel>)> {
    let peers = app.settings_model.peers_or_empty();
    if peers.is_empty() {
        return Vec::new();
    }

    let timeout = app.settings_model.peers_request_timeout();
    let mut tasks = Vec::with_capacity(peers.len());

    for peer in peers {
        let peer_url = peer.clone();
        tasks.push(tokio::spawn(
            async move { fetch_one_peer(peer_url, timeout).await },
        ));
    }

    let mut out = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok((instance, containers, hosts))) => out.push((instance, containers, hosts)),
            Ok(Err(err)) => eprintln!("peers_client::fanout_local_containers: {}", err),
            Err(err) => eprintln!("peers_client::fanout_local_containers: join error: {:?}", err),
        }
    }
    out
}

async fn fetch_one_peer(
    peer_url: String,
    timeout: Duration,
) -> Result<(String, Vec<ServiceInfo>, Vec<HostMemEntryHttpModel>), String> {
    let mut response = FlUrl::new(peer_url.as_str())
        .append_path_segment("api")
        .append_path_segment("containers")
        .append_path_segment("local")
        .set_timeout(timeout)
        .do_not_reuse_connection()
        .get()
        .await
        .map_err(|err| format!("peer {}: request failed: {:?}", peer_url, err))?;

    let status = response.get_status_code();
    if status != 200 {
        return Err(format!("peer {}: status {}", peer_url, status));
    }

    let body = response
        .get_body_as_slice()
        .await
        .map_err(|err| format!("peer {}: body read failed: {:?}", peer_url, err))?;

    let parsed: ContainersHttpResponse = serde_json::from_slice(body)
        .map_err(|err| format!("peer {}: parse failed: {}", peer_url, err))?;

    let containers = parsed
        .containers
        .into_iter()
        .map(|c| c.into_service_info())
        .collect();

    Ok((parsed.vm, containers, parsed.hosts))
}

/// Real-time log routing: try local first, then fan out to peers in parallel
/// and return the first 200 response. No cache lookup.
pub async fn fanout_logs(
    app: &AppContext,
    container_id: &str,
    lines: u32,
) -> RouteLogsResult {
    if container_owned_locally(app, container_id).await {
        let bytes = docker_sdk::sdk::get_container_logs(
            app.settings_model.docker_url.as_str(),
            container_id,
            lines,
        )
        .await;
        return RouteLogsResult::Ok(bytes);
    }

    let peers = app.settings_model.peers_or_empty();
    if peers.is_empty() {
        return RouteLogsResult::NotFound;
    }

    let timeout = app.settings_model.peers_request_timeout();
    let mut tasks = Vec::with_capacity(peers.len());

    for peer in peers {
        let peer_url = peer.clone();
        let id = container_id.to_string();
        tasks.push(tokio::spawn(async move {
            fetch_logs_from_peer(peer_url, id, lines, timeout).await
        }));
    }

    let mut last_err: Option<String> = None;
    for task in tasks {
        match task.await {
            Ok(Ok(bytes)) => return RouteLogsResult::Ok(bytes),
            Ok(Err(err)) => last_err = Some(err),
            Err(err) => last_err = Some(format!("join error: {:?}", err)),
        }
    }

    match last_err {
        Some(err) => RouteLogsResult::PeerError(err),
        None => RouteLogsResult::NotFound,
    }
}

async fn container_owned_locally(app: &AppContext, container_id: &str) -> bool {
    app.cache
        .get_snapshot()
        .await
        .iter()
        .any(|c| c.id == container_id)
}

/// Outcome of routing a per-process file-descriptor request.
pub enum RouteProcessesResult {
    Ok(Vec<ProcessHttpModel>),
    NotFound,
    PeerError(String),
}

/// Real-time process routing: serve from the local Docker host if the
/// container is owned here, otherwise fan out to peers in parallel and return
/// the first hit. Mirrors `fanout_logs`.
pub async fn fanout_processes(app: &AppContext, container_id: &str) -> RouteProcessesResult {
    if container_owned_locally(app, container_id).await {
        let processes = crate::proc_fd::collect_process_fd_list(
            app.settings_model.docker_url.as_str(),
            app.settings_model.host_proc_path(),
            container_id,
        )
        .await
        .into_iter()
        .map(|p| ProcessHttpModel {
            pid: p.pid,
            cmd: p.cmd,
            open_files: p.open_files,
            fd_limit: p.fd_limit,
            mem_rss: p.mem_rss,
            mem_vsize: p.mem_vsize,
            threads: p.threads,
        })
        .collect();

        return RouteProcessesResult::Ok(processes);
    }

    let peers = app.settings_model.peers_or_empty();
    if peers.is_empty() {
        return RouteProcessesResult::NotFound;
    }

    let timeout = app.settings_model.peers_request_timeout();
    let mut tasks = Vec::with_capacity(peers.len());

    for peer in peers {
        let peer_url = peer.clone();
        let id = container_id.to_string();
        tasks.push(tokio::spawn(async move {
            fetch_processes_from_peer(peer_url, id, timeout).await
        }));
    }

    let mut last_err: Option<String> = None;
    for task in tasks {
        match task.await {
            Ok(Ok(Some(processes))) => return RouteProcessesResult::Ok(processes),
            Ok(Ok(None)) => {}
            Ok(Err(err)) => last_err = Some(err),
            Err(err) => last_err = Some(format!("join error: {:?}", err)),
        }
    }

    match last_err {
        Some(err) => RouteProcessesResult::PeerError(err),
        None => RouteProcessesResult::NotFound,
    }
}

async fn fetch_processes_from_peer(
    peer_url: String,
    container_id: String,
    timeout: Duration,
) -> Result<Option<Vec<ProcessHttpModel>>, String> {
    let mut response = FlUrl::new(peer_url.as_str())
        .append_path_segment("api")
        .append_path_segment("containers")
        .append_path_segment("processes")
        .append_query_param("id", Some(container_id.as_str()))
        .set_timeout(timeout)
        .do_not_reuse_connection()
        .get()
        .await
        .map_err(|err| format!("peer {}: request failed: {:?}", peer_url, err))?;

    let status = response.get_status_code();
    if status == 404 {
        return Ok(None);
    }
    if status != 200 {
        return Err(format!("peer {}: status {}", peer_url, status));
    }

    let body = response
        .get_body_as_slice()
        .await
        .map_err(|err| format!("peer {}: body read failed: {:?}", peer_url, err))?;

    let parsed: ContainerProcessesHttpResponse = serde_json::from_slice(body)
        .map_err(|err| format!("peer {}: parse failed: {}", peer_url, err))?;

    Ok(Some(parsed.processes))
}

async fn fetch_logs_from_peer(
    peer_url: String,
    container_id: String,
    lines: u32,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let response = FlUrl::new(peer_url.as_str())
        .append_path_segment("api")
        .append_path_segment("containers")
        .append_path_segment("logs")
        .append_query_param("id", Some(container_id.as_str()))
        .append_query_param("lines_number", Some(lines.to_string()))
        .set_timeout(timeout)
        .get()
        .await
        .map_err(|err| format!("peer {}: request failed: {:?}", peer_url, err))?;

    let status = response.get_status_code();
    let body = response
        .receive_body()
        .await
        .map_err(|err| format!("peer {}: receive_body failed: {:?}", peer_url, err))?;

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
