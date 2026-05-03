use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EnvsHttpModel {
    pub envs: Vec<String>,
    pub request_pass_key: bool,
}

#[get("/api/envs")]
pub async fn get_envs() -> Result<EnvsHttpModel, ServerFnError> {
    let settings = crate::server::APP_CTX.settings_reader.get_settings().await;

    let user_id = dioxus::fullstack::FullstackContext::current()
        .and_then(|ctx| {
            let headers = ctx.parts_mut();
            headers
                .headers
                .get("x-ssl-user")
                .and_then(|user| user.to_str().ok())
                .map(|user| user.to_string())
        })
        .unwrap_or_default();

    let envs = settings.get_envs(&user_id);

    let mut request_pass_key = false;

    if settings.prompt_pass_phrase.unwrap_or(false) {
        if !crate::server::APP_CTX
            .ssh_private_key_resolver
            .private_key_is_loaded()
        {
            request_pass_key = true;
        }
    }
    Ok(EnvsHttpModel {
        envs,
        request_pass_key,
    })
}
