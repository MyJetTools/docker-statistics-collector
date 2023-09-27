use crate::app::AppContext;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use std::sync::Arc;

use super::contracts::*;

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/api/containers/running",
    description: "Get running containers info",
    summary: "Get running containers info",
    controller: "Containers",
    result:[
        {status_code: 200, description: "List of working containers", model:"ContainersHtpResponse" },
    ]
)]
pub struct GetRunningContainersAction {
    app: Arc<AppContext>,
}

impl GetRunningContainersAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetRunningContainersAction,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let containers = action.app.cache.get_snapshot().await;

    let response = ContainersHtpResponse {
        vm: action.app.settings_model.vm_name.clone(),
        containers: containers
            .into_iter()
            .filter(|itm| itm.running)
            .map(|itm| ContainerJsonModel {
                id: itm.id,
                image: itm.image,
                names: itm.names,
                enabled: itm.running,
                cpu: CpuUsageJsonMode {
                    usage: itm.cpu_usage,
                },
                mem: MemUsageJsonMode {
                    usage: itm.mem_usage,
                    available: itm.mem_available,
                    limit: itm.mem_limit,
                },
                labels: itm.labels,
            })
            .collect(),
    };

    HttpOutput::as_json(response).into_ok_result(false).into()
}
