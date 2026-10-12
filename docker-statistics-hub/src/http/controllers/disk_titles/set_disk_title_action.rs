use std::sync::Arc;

use my_http_server::{
    macros::{http_route, MyHttpInput},
    HttpContext, HttpFailResult, HttpOkResult, HttpOutput,
};
use serde::Serialize;

use crate::app::{normalize_disk_title, AppCtx};

#[http_route(
    method: "POST",
    route: "/api/disk-title",
    controller: "DiskTitles",
    description: "Names a host disk. The title is shown in the UI in place of the mount point and is kept in the disk-titles.yaml of this service; an empty title removes it.",
    summary: "Set or remove the title of a host disk",
    input_data: SetDiskTitleInputModel,
    result:[
        {status_code: 200, description: "The title as it was stored"},
    ]
)]
pub struct SetDiskTitleAction {
    app: Arc<AppCtx>,
}

impl SetDiskTitleAction {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

#[derive(MyHttpInput)]
pub struct SetDiskTitleInputModel {
    #[http_body(name = "env", description = "Environment name")]
    pub env: String,

    #[http_body(
        name = "vm",
        description = "VM / instance name (ENV_INFO) the disk belongs to"
    )]
    pub vm: String,

    #[http_body(
        name = "disk",
        description = "Mount point of the disk on that VM, e.g. /data"
    )]
    pub disk: String,

    #[http_body(
        name = "title",
        description = "Title to show for the disk. Empty or absent removes it"
    )]
    pub title: Option<String>,
}

#[derive(Serialize)]
pub struct SetDiskTitleResponse {
    /// The title as stored — trimmed; `null` once it has been removed.
    pub title: Option<String>,
}

async fn handle_request(
    action: &SetDiskTitleAction,
    input_data: SetDiskTitleInputModel,
    ctx: &mut HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let user_id = crate::auth::user_from_http(ctx);

    let settings = action.app.settings_reader.get_settings().await;
    if !settings.is_env_allowed_for_user(&user_id, &input_data.env) {
        return Err(HttpFailResult::as_forbidden(Some(format!(
            "env '{}' is not accessible for user '{}'",
            input_data.env, user_id
        ))));
    }
    drop(settings);

    if input_data.vm.is_empty() || input_data.disk.is_empty() {
        return Err(HttpFailResult::as_validation_error(
            "vm and disk are required",
        ));
    }

    let title = normalize_disk_title(input_data.title.as_deref())
        .map_err(HttpFailResult::as_validation_error)?;

    action
        .app
        .disk_titles
        .set(
            input_data.env.as_str(),
            input_data.vm.as_str(),
            input_data.disk.as_str(),
            title.clone(),
        )
        .await
        .map_err(HttpFailResult::as_fatal_error)?;

    println!(
        "Disk title of {}/{}:{} set to {:?} by '{}'",
        input_data.env, input_data.vm, input_data.disk, title, user_id
    );

    HttpOutput::as_json(SetDiskTitleResponse { title })
        .into_ok_result(false)
        .into()
}
