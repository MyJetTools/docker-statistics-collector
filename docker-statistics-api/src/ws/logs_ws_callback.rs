use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use my_http_server::web_sockets::{
    MyWebSocket, MyWebSocketCallback, MyWebSocketHttpRequest, WebSocketConnectedFail, WsMessage,
};
use tokio_tungstenite::tungstenite::Message;

use crate::app::AppCtx;

pub struct LogsWsCallback {
    app: Arc<AppCtx>,
}

impl LogsWsCallback {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl MyWebSocketCallback for LogsWsCallback {
    async fn connected(
        &self,
        ws: Arc<MyWebSocket>,
        _http_request: MyWebSocketHttpRequest,
        _disconnect_timeout: Duration,
    ) -> Result<(), WebSocketConnectedFail> {
        let (env, container_id, tail) = {
            let query = match ws.get_query_string() {
                Some(q) => q,
                None => {
                    return Err(WebSocketConnectedFail {
                        reason: "missing query string".to_string(),
                        write_to_logs: false,
                    });
                }
            };

            let env = match query.get_required("env").and_then(|v| v.as_string()) {
                Ok(s) => s,
                Err(_) => {
                    return Err(WebSocketConnectedFail {
                        reason: "missing 'env' query param".to_string(),
                        write_to_logs: false,
                    });
                }
            };
            let id = match query.get_required("id").and_then(|v| v.as_string()) {
                Ok(s) => s,
                Err(_) => {
                    return Err(WebSocketConnectedFail {
                        reason: "missing 'id' query param".to_string(),
                        write_to_logs: false,
                    });
                }
            };
            let tail = query
                .get_optional("tail")
                .and_then(|v| v.from_str::<u32>().ok());
            (env, id, tail)
        };

        let app = self.app.clone();
        let ws_for_task = ws.clone();
        tokio::spawn(async move {
            forward_logs(app, env, container_id, tail, ws_for_task).await;
        });

        Ok(())
    }

    async fn disconnected(&self, _ws: &MyWebSocket) {}

    async fn on_message(&self, _ws: Arc<MyWebSocket>, _message: WsMessage) {}
}

async fn forward_logs(
    app: Arc<AppCtx>,
    env: String,
    container_id: String,
    tail: Option<u32>,
    ws: Arc<MyWebSocket>,
) {
    println!("[api-ws-logs] client connected env={env} id={container_id} tail={tail:?}");
    let settings = app.settings_reader.get_settings().await;
    let master_url = match settings.envs.get(&env) {
        Some(vm) => vm.url.clone(),
        None => {
            eprintln!("[api-ws-logs] env '{env}' not in settings — known envs: {:?}",
                settings.envs.keys().collect::<Vec<_>>());
            send_error(&ws, &format!("env '{env}' not configured")).await;
            ws.disconnect().await;
            return;
        }
    };

    let upstream_url = build_collector_ws_url(&master_url, &container_id, tail);
    println!("[api-ws-logs] dialing collector upstream={upstream_url}");
    let upstream = match tokio_tungstenite::connect_async(&upstream_url).await {
        Ok((stream, _)) => {
            println!("[api-ws-logs] upstream connected env={env} id={container_id}");
            stream
        }
        Err(err) => {
            eprintln!("[api-ws-logs] upstream connect FAILED to {upstream_url}: {err:?}");
            send_error(
                &ws,
                &format!("failed to open WS to collector {master_url}: {err:?}"),
            )
            .await;
            ws.disconnect().await;
            return;
        }
    };

    let (_write_upstream, mut read_upstream) = upstream.split();

    // Periodic tick so we notice a client disconnect even if the upstream
    // collector is silent for a long stretch — otherwise we'd leak the
    // upstream connection until docker pushes the next log line.
    let mut alive_check = tokio::time::interval(std::time::Duration::from_secs(2));
    alive_check.tick().await; // skip the immediate first tick

    loop {
        if !ws.is_connected() {
            break;
        }
        tokio::select! {
            biased;
            _ = alive_check.tick() => {
                continue;
            }
            msg = read_upstream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        ws.send_message(std::iter::once(WsMessage::Text(text.to_string().into())))
                            .await;
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        ws.send_message(std::iter::once(WsMessage::Binary(bytes.to_vec().into())))
                            .await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        println!("api ws upstream error for {container_id}: {err:?}");
                        break;
                    }
                }
            }
        }
    }

    ws.disconnect().await;
}

fn build_collector_ws_url(master_http_url: &str, container_id: &str, tail: Option<u32>) -> String {
    let scheme = if master_http_url.starts_with("https://") {
        "wss"
    } else {
        "ws"
    };
    let stripped = master_http_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    let mut url = format!("{scheme}://{stripped}/ws/logs?id={container_id}");
    if let Some(t) = tail {
        url.push_str(&format!("&tail={t}"));
    }
    url
}

async fn send_error(ws: &Arc<MyWebSocket>, msg: &str) {
    let escaped = serde_json::to_string(msg).unwrap_or_else(|_| "\"\"".to_string());
    let payload = format!(r#"{{"error":{escaped}}}"#);
    ws.send_message(std::iter::once(WsMessage::Text(payload.into())))
        .await;
}
