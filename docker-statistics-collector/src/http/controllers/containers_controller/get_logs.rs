use crate::app::AppContext;

use my_http_server::{HttpContext, HttpFailResult, HttpOkResult, HttpOutput};

use std::sync::Arc;

use super::contracts::*;

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/api/containers/logs",
    description: "Get containers logs",
    summary: "Get containers logs",
    controller: "Containers",
    result:[
        {status_code: 200, description: "Logs of container", model:"String" },
    ]
)]
pub struct GetLogsAction {
    app: Arc<AppContext>,
}

impl GetLogsAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetLogsAction,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let containers = action.app.cache.get_snapshot().await;

    let response = ContainersHtpResponse {
        vm: action.app.settings_model.vm_name.clone(),
        containers: containers
            .into_iter()
            .map(|itm| ContainerJsonModel {
                id: itm.id,
                image: itm.image,
                enabled: itm.running,
                cpu: CpuUsageJsonMode {
                    usage: itm.cpu_usage,
                },
                mem: MemUsageJsonMode {
                    usage: itm.mem_usage,
                    available: itm.mem_available,
                    limit: itm.mem_limit,
                },
                names: itm.names,
                labels: itm.labels,
            })
            .collect(),
    };

    HttpOutput::as_json(response).into_ok_result(false).into()
}
