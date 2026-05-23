use std::sync::Arc;

use my_http_server::{
    macros::{http_route, MyHttpInput},
    HttpContext, HttpFailResult, HttpOkResult, HttpOutput,
};

use crate::app::AppCtx;
use crate::models::LogLineHttpModel;

#[http_route(
    method: "GET",
    route: "/api/logs",
    controller: "Logs",
    description: "Proxies container logs from the env's master collector",
    summary: "Read container logs",
    input_data: GetLogsInputModel,
    result:[
        {status_code: 200, description: "Array of parsed log lines"},
    ]
)]
pub struct GetLogsAction {
    app: Arc<AppCtx>,
}

impl GetLogsAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

#[derive(MyHttpInput)]
pub struct GetLogsInputModel {
    #[http_query(name = "env", description = "Environment name")]
    pub env: String,

    #[http_query(name = "url", description = "Master URL for the env")]
    pub url: String,

    #[http_query(name = "id", description = "Container id")]
    pub id: String,

    #[http_query(name = "lines_amount", description = "Number of log lines to fetch")]
    pub lines_amount: u32,
}

async fn handle_request(
    action: &GetLogsAction,
    input_data: GetLogsInputModel,
    _ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let fl_url = action
        .app
        .get_fl_url(input_data.env.as_str(), input_data.url.as_str())
        .await;

    let payload = crate::http_client::get_logs(fl_url, input_data.id, input_data.lines_amount)
        .await
        .map_err(|err| HttpFailResult::as_fatal_error(err))?;

    let result = parse_logs_payload(payload);

    HttpOutput::as_json(result).into_ok_result(false).into()
}

fn parse_logs_payload(payload: Vec<u8>) -> Vec<LogLineHttpModel> {
    if payload.is_empty() {
        return vec![];
    }

    let mut result = Vec::new();

    let mut payload = payload.into_iter();
    loop {
        let tp = payload.next();

        if tp.is_none() {
            break;
        }

        let tp = tp.unwrap();

        let n = payload.next().unwrap_or(255);
        if n != 0 {
            break;
        }
        let n = payload.next().unwrap_or(255);
        if n != 0 {
            break;
        }

        payload.next().unwrap_or(255);
        if n != 0 {
            break;
        }

        let mut size = [0u8; 4];

        size[0] = payload.next().unwrap();
        size[1] = payload.next().unwrap();
        size[2] = payload.next().unwrap();
        size[3] = payload.next().unwrap();

        let size = u32::from_be_bytes(size) as usize;

        let mut str = Vec::with_capacity(size);

        for _ in 0..size - 1 {
            str.push(payload.next().unwrap());
        }

        payload.next().unwrap();

        let item = LogLineHttpModel {
            tp,
            line: String::from_utf8(str).unwrap(),
        };

        result.push(item);
    }

    result
}
