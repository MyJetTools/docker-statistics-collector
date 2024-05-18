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
        {status_code: 200, description: "List of containers", model:"ContainersHtpResponse" },
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
            .map(|itm| ContainerJsonModel::new(itm))
            .collect(),
    };

    HttpOutput::as_json(response).into_ok_result(false).into()
}
