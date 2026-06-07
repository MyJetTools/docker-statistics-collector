use std::time::Duration;

use flurl::body::FlUrlBody;
use flurl::IntoFlUrl;
use serde::*;

#[derive(Serialize)]
struct ExecCreateBody {
    #[serde(rename = "AttachStdout")]
    attach_stdout: bool,
    #[serde(rename = "AttachStderr")]
    attach_stderr: bool,
    #[serde(rename = "Cmd")]
    cmd: Vec<String>,
}

#[derive(Deserialize)]
struct ExecCreateResponse {
    #[serde(rename = "Id")]
    id: String,
}

#[derive(Serialize)]
struct ExecStartBody {
    #[serde(rename = "Detach")]
    detach: bool,
    #[serde(rename = "Tty")]
    tty: bool,
}

#[derive(Deserialize)]
struct ExecInspectResponse {
    #[serde(rename = "ExitCode")]
    exit_code: Option<i64>,
}

/// Result of running a command inside a container.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecResult {
    /// Combined stdout/stderr with Docker's multiplexed framing stripped.
    pub output: String,
    /// Process exit code (`None` if it couldn't be read).
    pub exit_code: Option<i64>,
}

/// Run `command` inside a container via the Docker exec API, as `sh -c
/// "<command>"`, and return the combined stdout/stderr plus the exit code.
///
/// Three Docker calls: create exec → start (non-detached, returns the output
/// stream) → inspect (for the exit code).
pub async fn exec_in_container(
    url: &str,
    container_id: &str,
    command: &str,
) -> Result<ExecResult, String> {
    // 1) Create the exec instance.
    let create_body = ExecCreateBody {
        attach_stdout: true,
        attach_stderr: true,
        cmd: vec!["sh".to_string(), "-c".to_string(), command.to_string()],
    };

    let mut create_resp = url
        .append_path_segment("containers")
        .append_path_segment(container_id)
        .append_path_segment("exec")
        .with_header("host", "docker")
        .with_header("connection", "close")
        .set_timeout(Duration::from_secs(60))
        .post(FlUrlBody::as_json(&create_body))
        .await
        .map_err(|err| format!("exec create failed: {:?}", err))?;

    if create_resp.get_status_code() != 201 {
        let status = create_resp.get_status_code();
        let body = create_resp.get_body_as_slice().await.unwrap_or(&[]);
        return Err(format!(
            "exec create returned {}: {}",
            status,
            String::from_utf8_lossy(body)
        ));
    }

    let created: ExecCreateResponse = create_resp
        .get_json()
        .await
        .map_err(|err| format!("exec create parse failed: {:?}", err))?;

    // 2) Start it (non-detached) — the response body is the multiplexed output.
    let start_body = ExecStartBody {
        detach: false,
        tty: false,
    };

    let start_resp = url
        .append_path_segment("exec")
        .append_path_segment(created.id.as_str())
        .append_path_segment("start")
        .with_header("host", "docker")
        .with_header("connection", "close")
        .set_timeout(Duration::from_secs(60))
        .post(FlUrlBody::as_json(&start_body))
        .await
        .map_err(|err| format!("exec start failed: {:?}", err))?;

    let bytes = start_resp
        .receive_body()
        .await
        .map_err(|err| format!("exec body read failed: {:?}", err))?;

    let output = demux_stream(&bytes);

    // 3) Inspect for the exit code (best-effort).
    let exit_code = match url
        .append_path_segment("exec")
        .append_path_segment(created.id.as_str())
        .append_path_segment("json")
        .with_header("host", "docker")
        .with_header("connection", "close")
        .set_timeout(Duration::from_secs(5))
        .get()
        .await
    {
        Ok(mut r) => r
            .get_json::<ExecInspectResponse>()
            .await
            .ok()
            .and_then(|i| i.exit_code),
        Err(_) => None,
    };

    Ok(ExecResult { output, exit_code })
}

/// Strip Docker's 8-byte multiplexed frame headers (`[stream, 0,0,0, len_be4]`)
/// from a non-TTY exec/log stream. Falls back to a lossy decode if the stream
/// isn't framed (TTY mode).
fn demux_stream(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        let stream_type = bytes[i];
        if stream_type > 2 || bytes[i + 1] != 0 || bytes[i + 2] != 0 || bytes[i + 3] != 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        let len =
            u32::from_be_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        let start = i + 8;
        let end = start + len;
        if end > bytes.len() {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        out.push_str(&String::from_utf8_lossy(&bytes[start..end]));
        i = end;
    }
    if i != bytes.len() {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    out
}
