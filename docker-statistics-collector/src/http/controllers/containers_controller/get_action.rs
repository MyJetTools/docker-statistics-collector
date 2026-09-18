use crate::app::AppContext;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use std::sync::Arc;

use super::contracts::*;

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/api/containers",
    description: "Get containers info",
    summary: "Get containers info",
    controller: "Containers",
    result:[
        {status_code: 200, description: "List of containers", model:"ContainersHttpResponse" },
    ]
)]
pub struct GetContainersAction {
    app: Arc<AppContext>,
}

impl GetContainersAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetContainersAction,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let local_instance = action.app.get_env_info();

    // Local host memory + physical disks (host-level, not per-container).
    let proc_base = action.app.settings_model.host_proc_path().to_string();
    let root_base = action.app.settings_model.host_root_path().to_string();
    let ignore_disks = action.app.settings_model.ignore_disks().to_vec();

    // All three are independent and each costs a full live Docker scan on some host.
    // Awaiting them in sequence used to add the local scan to every peer's, roughly
    // doubling the wall-clock of the api's poll for no reason.
    let (local, local_host, peers) = tokio::join!(
        action.app.live.get_snapshot(&action.app.disk_sizes),
        tokio::task::spawn_blocking(move || {
            let mem = crate::host_mem::read(&proc_base);
            let disks = crate::host_disks::read(&proc_base, &root_base, &ignore_disks);
            (mem, disks)
        }),
        crate::peers_client::fanout_local_containers(&action.app),
    );

    let local = local.map_err(HttpFailResult::as_fatal_error)?;

    let mut containers: Vec<ContainerJsonModel> = local
        .into_iter()
        .map(|itm| ContainerJsonModel::new(itm, local_instance.clone()))
        .collect();

    let mut hosts: Vec<HostMemEntryHttpModel> = Vec::new();

    if let Ok((Some(snap), disks)) = local_host {
        hosts.push(HostMemEntryHttpModel::from_snapshot(
            local_instance.clone(),
            snap,
            disks,
        ));
    }

    // Peers — containers + their host memory.
    for (peer_instance, peer_containers, peer_hosts) in peers {
        for itm in peer_containers {
            containers.push(ContainerJsonModel::new(itm, peer_instance.clone()));
        }
        hosts.extend(peer_hosts);
    }

    let response = ContainersHttpResponse {
        vm: local_instance,
        containers,
        hosts,
    };

    HttpOutput::as_json(response).into_ok_result(false).into()
}
