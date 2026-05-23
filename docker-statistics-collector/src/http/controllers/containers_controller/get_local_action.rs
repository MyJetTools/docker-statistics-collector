use crate::app::AppContext;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use std::sync::Arc;

use super::contracts::*;

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/api/containers/local",
    description: "Get this instance's containers only (used for federation between peers)",
    summary: "Get local containers info",
    controller: "Containers",
    result:[
        {status_code: 200, description: "List of local containers", model:"ContainersHttpResponse" },
    ]
)]
pub struct GetLocalContainersAction {
    app: Arc<AppContext>,
}

impl GetLocalContainersAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetLocalContainersAction,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let containers = action.app.cache.get_snapshot().await;
    let instance = action.app.get_env_info();
    let proc_base = action.app.settings_model.host_proc_path().to_string();

    let host_mem = tokio::task::spawn_blocking(move || crate::host_mem::read(&proc_base))
        .await
        .ok()
        .flatten();

    let mut hosts = Vec::new();
    if let Some(snap) = host_mem {
        hosts.push(HostMemEntryHttpModel::from_snapshot(instance.clone(), snap));
    }

    let response = ContainersHttpResponse {
        vm: instance.clone(),
        containers: containers
            .into_iter()
            .map(|itm| ContainerJsonModel::new(itm, instance.clone()))
            .collect(),
        hosts,
    };

    HttpOutput::as_json(response).into_ok_result(false).into()
}
