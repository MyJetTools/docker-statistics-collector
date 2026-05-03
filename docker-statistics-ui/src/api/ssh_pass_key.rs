use dioxus::prelude::*;

#[post("/api/pass_phrase")]
pub async fn apply_pass_phrase(pass_phrase: String) -> Result<(), ServerFnError> {
    crate::server::APP_CTX
        .ssh_private_key_resolver
        .set_pass_phrase(pass_phrase)
        .await;

    Ok(())
}
