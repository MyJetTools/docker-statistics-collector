use std::time::Duration;

use flurl::FlUrl;

use crate::app::{AppContext, ServiceInfo};
use crate::http::controllers::containers_controller::contracts::{
    ContainerExecHttpResponse, ContainerProcessesHttpResponse, ContainersHttpResponse,
    HostMemEntryHttpModel, ProcessHttpModel,
};
use crate::http::controllers::containers_controller::RouteLogsResult;
use crate::http::controllers::exec_permission_controller::contracts::ExecPermissionHttpResponse;

use docker_sdk::exec::ExecResult;

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

/// Outcome of routing a container exec request.
pub enum RouteExecResult {
    Ok(ExecResult),
    NotFound,
    /// The owning instance refused: MCP exec is currently locked there.
    Forbidden(String),
    PeerError(String),
}

/// Who asked for the exec.
///
/// `Mcp` is the AI-agent surface and is gated behind [`ExecPermission`]. `Trusted`
/// covers the human surfaces (UI exec console, the HTTP endpoint behind the api's
/// `x-ssl-user` auth) and is never gated.
///
/// The origin travels with the request all the way to the instance that owns the
/// container, so the gate is always evaluated by the machine that actually runs
/// the command — never by the one that merely forwards it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExecOrigin {
    Mcp,
    Trusted,
}

impl ExecOrigin {
    pub fn from_mcp_flag(is_mcp: Option<bool>) -> Self {
        match is_mcp {
            Some(true) => Self::Mcp,
            _ => Self::Trusted,
        }
    }

    pub fn is_mcp(&self) -> bool {
        *self == Self::Mcp
    }
}

/// Exec a command in a container: run locally if the container is owned here,
/// otherwise fan out to peers and return the first hit. Mirrors
/// `fanout_processes`. Exec can run long, so peer calls use a 65s timeout.
///
/// MCP-originated calls are rejected unless exec has been unlocked on the
/// instance that owns the container.
pub async fn fanout_exec(
    app: &AppContext,
    container_id: &str,
    command: &str,
    origin: ExecOrigin,
) -> RouteExecResult {
    if container_owned_locally(app, container_id).await {
        if origin.is_mcp() && !app.exec_permission.is_enabled() {
            return RouteExecResult::Forbidden(exec_locked_message(app));
        }

        return match docker_sdk::exec::exec_in_container(
            app.settings_model.docker_url.as_str(),
            container_id,
            command,
        )
        .await
        {
            Ok(result) => RouteExecResult::Ok(result),
            Err(err) => RouteExecResult::PeerError(err),
        };
    }

    let peers = app.settings_model.peers_or_empty();
    if peers.is_empty() {
        return RouteExecResult::NotFound;
    }

    let timeout = Duration::from_secs(65);
    let mut tasks = Vec::with_capacity(peers.len());

    for peer in peers {
        let peer_url = peer.clone();
        let id = container_id.to_string();
        let cmd = command.to_string();
        tasks.push(tokio::spawn(async move {
            fetch_exec_from_peer(peer_url, id, cmd, origin, timeout).await
        }));
    }

    let mut last_err: Option<String> = None;
    let mut forbidden: Option<String> = None;
    for task in tasks {
        match task.await {
            Ok(Ok(PeerExecOutcome::Ok(result))) => return RouteExecResult::Ok(result),
            Ok(Ok(PeerExecOutcome::NotFound)) => {}
            Ok(Ok(PeerExecOutcome::Forbidden(msg))) => forbidden = Some(msg),
            Ok(Err(err)) => last_err = Some(err),
            Err(err) => last_err = Some(format!("join error: {:?}", err)),
        }
    }

    // A 403 means we *did* reach the owner and it refused — that is a far more
    // useful answer than any transport error from the peers that didn't own it.
    if let Some(msg) = forbidden {
        return RouteExecResult::Forbidden(msg);
    }

    match last_err {
        Some(err) => RouteExecResult::PeerError(err),
        None => RouteExecResult::NotFound,
    }
}

fn exec_locked_message(app: &AppContext) -> String {
    let minutes = app.settings_model.exec_unlock_duration().as_secs() / 60;
    format!(
        "exec_in_container is DISABLED on instance '{}'. It is a human-gated operation: ask the user to open the Docker Statistics UI for this VM and press \"Enable exec\", which unlocks it for {} minutes. Do not retry until they confirm.",
        app.get_env_info(),
        minutes
    )
}

enum PeerExecOutcome {
    Ok(ExecResult),
    NotFound,
    Forbidden(String),
}

async fn fetch_exec_from_peer(
    peer_url: String,
    container_id: String,
    command: String,
    origin: ExecOrigin,
    timeout: Duration,
) -> Result<PeerExecOutcome, String> {
    let mut request = FlUrl::new(peer_url.as_str())
        .append_path_segment("api")
        .append_path_segment("containers")
        .append_path_segment("exec")
        .append_query_param("id", Some(container_id.as_str()))
        .append_query_param("command", Some(command.as_str()));

    // Carry the origin across the hop so the owning peer applies its own gate.
    if origin.is_mcp() {
        request = request.append_query_param("mcp", Some("true"));
    }

    let mut response = request
        .set_timeout(timeout)
        .do_not_reuse_connection()
        .post(flurl::body::FlUrlBody::Empty)
        .await
        .map_err(|err| format!("peer {}: request failed: {:?}", peer_url, err))?;

    let status = response.get_status_code();
    if status == 404 {
        return Ok(PeerExecOutcome::NotFound);
    }
    if status == 403 {
        let body = response
            .get_body_as_slice()
            .await
            .map_err(|err| format!("peer {}: body read failed: {:?}", peer_url, err))?;
        return Ok(PeerExecOutcome::Forbidden(
            String::from_utf8_lossy(body).into_owned(),
        ));
    }
    if status != 200 {
        return Err(format!("peer {}: status {}", peer_url, status));
    }

    let body = response
        .get_body_as_slice()
        .await
        .map_err(|err| format!("peer {}: body read failed: {:?}", peer_url, err))?;

    let parsed: ContainerExecHttpResponse = serde_json::from_slice(body)
        .map_err(|err| format!("peer {}: parse failed: {}", peer_url, err))?;

    Ok(PeerExecOutcome::Ok(ExecResult {
        output: parsed.output,
        exit_code: parsed.exit_code,
    }))
}

/// What to do with an instance's exec-permission window.
#[derive(Clone, Copy)]
pub enum ExecPermissionCommand {
    Status,
    Enable,
    Disable,
}

impl ExecPermissionCommand {
    fn path_segment(&self) -> Option<&'static str> {
        match self {
            Self::Status => None,
            Self::Enable => Some("enable"),
            Self::Disable => Some("disable"),
        }
    }
}

pub struct ExecPermissionState {
    pub instance: String,
    pub enabled: bool,
    pub seconds_left: i64,
}

pub enum RouteExecPermissionResult {
    Ok(ExecPermissionState),
    InstanceNotFound,
    PeerError(String),
}

/// Applies `command` to the instance named `instance` — this collector, or the
/// peer whose `ENV_INFO` matches. `instance = None` always means "this one".
///
/// Peers are addressed by broadcasting the request to all of them with
/// `no_forward=true`; every collector answers only for its own `ENV_INFO` and
/// 404s otherwise. That avoids maintaining a url→instance map, and the
/// `no_forward` flag makes re-broadcast loops impossible.
pub async fn route_exec_permission(
    app: &AppContext,
    instance: Option<&str>,
    command: ExecPermissionCommand,
    by_user: &str,
    allow_forward: bool,
) -> RouteExecPermissionResult {
    let me = app.get_env_info();

    let targets_me = match instance {
        None => true,
        Some(name) => name.is_empty() || name == me,
    };

    if targets_me {
        return RouteExecPermissionResult::Ok(apply_exec_permission_locally(app, command, by_user));
    }

    if !allow_forward {
        return RouteExecPermissionResult::InstanceNotFound;
    }

    let instance = instance.unwrap();
    let peers = app.settings_model.peers_or_empty();
    if peers.is_empty() {
        return RouteExecPermissionResult::InstanceNotFound;
    }

    let timeout = app.settings_model.peers_request_timeout();
    let mut tasks = Vec::with_capacity(peers.len());

    for peer in peers {
        let peer_url = peer.clone();
        let name = instance.to_string();
        let user = by_user.to_string();
        tasks.push(tokio::spawn(async move {
            fetch_exec_permission_from_peer(peer_url, name, command, user, timeout).await
        }));
    }

    let mut last_err: Option<String> = None;
    for task in tasks {
        match task.await {
            Ok(Ok(Some(state))) => return RouteExecPermissionResult::Ok(state),
            Ok(Ok(None)) => {}
            Ok(Err(err)) => last_err = Some(err),
            Err(err) => last_err = Some(format!("join error: {:?}", err)),
        }
    }

    match last_err {
        Some(err) => RouteExecPermissionResult::PeerError(err),
        None => RouteExecPermissionResult::InstanceNotFound,
    }
}

fn apply_exec_permission_locally(
    app: &AppContext,
    command: ExecPermissionCommand,
    by_user: &str,
) -> ExecPermissionState {
    let instance = app.get_env_info();
    let user = if by_user.is_empty() {
        "<unknown>"
    } else {
        by_user
    };

    let status = match command {
        ExecPermissionCommand::Status => app.exec_permission.get_status(),
        ExecPermissionCommand::Enable => {
            let duration = app.settings_model.exec_unlock_duration();
            // Unlocking arbitrary command execution is worth an audit line.
            println!(
                "[exec-permission] ENABLED on '{}' for {}s by '{}'",
                instance,
                duration.as_secs(),
                user
            );
            app.exec_permission.enable_for(duration)
        }
        ExecPermissionCommand::Disable => {
            println!("[exec-permission] DISABLED on '{}' by '{}'", instance, user);
            app.exec_permission.disable()
        }
    };

    ExecPermissionState {
        instance,
        enabled: status.enabled,
        seconds_left: status.seconds_left,
    }
}

async fn fetch_exec_permission_from_peer(
    peer_url: String,
    instance: String,
    command: ExecPermissionCommand,
    by_user: String,
    timeout: Duration,
) -> Result<Option<ExecPermissionState>, String> {
    let mut request = FlUrl::new(peer_url.as_str())
        .append_path_segment("api")
        .append_path_segment("exec-permission");

    if let Some(segment) = command.path_segment() {
        request = request.append_path_segment(segment);
    }

    let request = request
        .append_query_param("instance", Some(instance.as_str()))
        .append_query_param("no_forward", Some("true"))
        .append_query_param("by", Some(by_user.as_str()))
        .set_timeout(timeout)
        .do_not_reuse_connection();

    let mut response = match command {
        ExecPermissionCommand::Status => request.get().await,
        _ => request.post(flurl::body::FlUrlBody::Empty).await,
    }
    .map_err(|err| format!("peer {}: request failed: {:?}", peer_url, err))?;

    let status = response.get_status_code();
    if status == 404 {
        return Ok(None); // not this peer's instance
    }
    if status != 200 {
        return Err(format!("peer {}: status {}", peer_url, status));
    }

    let body = response
        .get_body_as_slice()
        .await
        .map_err(|err| format!("peer {}: body read failed: {:?}", peer_url, err))?;

    let parsed: ExecPermissionHttpResponse = serde_json::from_slice(body)
        .map_err(|err| format!("peer {}: parse failed: {}", peer_url, err))?;

    Ok(Some(ExecPermissionState {
        instance: parsed.instance,
        enabled: parsed.enabled,
        seconds_left: parsed.seconds_left,
    }))
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
