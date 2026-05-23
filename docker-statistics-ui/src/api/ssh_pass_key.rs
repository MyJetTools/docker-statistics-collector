use serde::Serialize;

use crate::models::RequestError;

use super::get_base_url;

#[derive(Serialize)]
struct ApplyPassPhraseRequest {
    pass_phrase: String,
}

pub async fn apply_pass_phrase(pass_phrase: String) -> Result<(), RequestError> {
    let url = format!("{}/api/pass_phrase", get_base_url());
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&ApplyPassPhraseRequest { pass_phrase })
        .send()
        .await?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(RequestError {
            message: format!("Failed to apply pass phrase: HTTP {}", resp.status()),
        })
    }
}
