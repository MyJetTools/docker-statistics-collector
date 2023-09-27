use crate::app::AppContext;

use my_http_server::{
    macros::MyHttpObjectStructure, HttpContext, HttpFailResult, HttpOkResult, HttpOutput,
};
use serde::Serialize;

use std::sync::Arc;

#[my_http_server::macros::http_route(
    method: "GET",
    route: "/api/containers",
    description: "Get containers info",
    summary: "Get containers info",
    controller: "Containers",
    result:[
        {status_code: 200, description: "List of working containers", model:"ContainersHtpResponse" },
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
                },
            })
            .collect(),
    };

    HttpOutput::as_json(response).into_ok_result(false).into()
}

#[derive(MyHttpObjectStructure, Serialize)]
pub struct ContainersHtpResponse {
    pub vm: String,
    pub containers: Vec<ContainerJsonModel>,
}

#[derive(Serialize, MyHttpObjectStructure)]
pub struct ContainerJsonModel {
    pub id: String,
    pub image: String,
    pub enabled: bool,
    pub cpu: CpuUsageJsonMode,
    pub mem: MemUsageJsonMode,
}
#[derive(Serialize, MyHttpObjectStructure)]
pub struct CpuUsageJsonMode {
    pub usage: Option<f64>,
}

#[derive(Serialize, MyHttpObjectStructure)]
pub struct MemUsageJsonMode {
    pub usage: Option<i64>,
    pub available: Option<i64>,
}
