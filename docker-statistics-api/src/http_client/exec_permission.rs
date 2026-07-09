use flurl::FlUrl;
use serde::{Deserialize, Serialize};

/// Exec-permission window of one collector instance, as reported by its
/// `/api/exec-permission*` endpoints.
#[derive(Deserialize, Serialize, Debug)]
pub struct ExecPermissionModel {
    pub instance: String,
    pub enabled: bool,
    pub seconds_left: i64,
}

#[derive(Clone, Copy)]
pub enum ExecPermissionAction {
    Status,
    Enable,
    Disable,
}

impl ExecPermissionAction {
    fn path_segment(&self) -> Option<&'static str> {
        match self {
            Self::Status => None,
            Self::Enable => Some("enable"),
            Self::Disable => Some("disable"),
        }
    }
}

/// Talks to the env's master collector, which routes the call on to the peer
/// owning `instance` when that is not the master itself.
pub async fn exec_permission(
    fl_url: FlUrl,
    instance: String,
    by_user: String,
    action: ExecPermissionAction,
) -> Result<ExecPermissionModel, String> {
    let mut request = fl_url
        .append_path_segment("api")
        .append_path_segment("exec-permission");

    if let Some(segment) = action.path_segment() {
        request = request.append_path_segment(segment);
    }

    let request = request
        .append_query_param("instance", Some(instance))
        // The collector prefers the x-ssl-user header, but we reach it over a
        // plain server-to-server call, so hand the identity across explicitly.
        .append_query_param("by", Some(by_user));

    let response = match action {
        ExecPermissionAction::Status => request.get().await,
        _ => request.post(flurl::body::FlUrlBody::Empty).await,
    };

    let mut response = match response {
        Ok(response) => response,
        Err(err) => return Err(format!("Error: {:?}", err)),
    };

    let status_code = response.get_status_code();

    let body = match response.get_body_as_slice().await {
        Ok(body) => body,
        Err(err) => return Err(format!("Error reading body: {:?}", err)),
    };

    if status_code == 404 {
        // The collector answers 404 with a plain-text explanation — pass it through.
        return Err(String::from_utf8_lossy(body).into_owned());
    }

    if status_code != 200 {
        return Err(format!("Collector returned status {}", status_code));
    }

    match serde_json::from_slice::<ExecPermissionModel>(body) {
        Ok(parsed) => Ok(parsed),
        Err(err) => Err(format!("Parse error: {}", err)),
    }
}
