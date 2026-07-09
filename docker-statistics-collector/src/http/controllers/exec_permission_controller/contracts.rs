use my_http_server::macros::{MyHttpInput, MyHttpObjectStructure};
use serde::{Deserialize, Serialize};

/// Shared by the status/enable/disable actions — they only differ in the verb.
#[derive(MyHttpInput)]
pub struct ExecPermissionHttpInput {
    #[http_query(description:"Target instance (ENV_INFO). Defaults to this collector.")]
    pub instance: Option<String>,

    #[http_query(description:"Internal: set when a peer forwards the call, so it is not re-broadcast")]
    pub no_forward: Option<bool>,

    #[http_query(description:"Internal: user who initiated the change, forwarded for the audit log")]
    pub by: Option<String>,
}

/// The exec-permission window of one instance:
/// * `instance`     — ENV_INFO of the instance this window belongs to
/// * `enabled`      — true while MCP `exec_in_container` is unlocked there
/// * `seconds_left` — seconds until the window closes; 0 when disabled
///
/// Field-level doc comments are deliberately avoided — `MyHttpObjectStructure`
/// panics on the `#[doc = "..."]` attributes they expand into.
#[derive(MyHttpObjectStructure, Serialize, Deserialize)]
pub struct ExecPermissionHttpResponse {
    pub instance: String,
    pub enabled: bool,
    pub seconds_left: i64,
}
