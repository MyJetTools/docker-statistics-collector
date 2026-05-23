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
    let local = action.app.cache.get_snapshot().await;
    let local_instance = action.app.get_env_info();

    let mut containers: Vec<ContainerJsonModel> = local
        .into_iter()
        .map(|itm| ContainerJsonModel::new(itm, local_instance.clone()))
        .collect();

    let mut hosts: Vec<HostMemEntryHttpModel> = Vec::new();

    // Local host memory.
    let proc_base = action.app.settings_model.host_proc_path().to_string();
    let local_host_mem = tokio::task::spawn_blocking(move || crate::host_mem::read(&proc_base))
        .await
        .ok()
        .flatten();
    if let Some(snap) = local_host_mem {
        hosts.push(HostMemEntryHttpModel::from_snapshot(
            local_instance.clone(),
            snap,
        ));
    }

    // Peers — containers + their host memory.
    for (peer_instance, peer_containers, peer_hosts) in
        crate::peers_client::fanout_local_containers(&action.app).await
    {
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
