use std::collections::HashMap;
use std::sync::Mutex;
use std::{sync::Arc, time::Duration};

use futures::{SinkExt, StreamExt};
use my_http_server::web_sockets::{
    MyWebSocket, MyWebSocketCallback, MyWebSocketHttpRequest, WebSocketConnectedFail, WsMessage,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

use crate::app::AppCtx;

/// Bidirectional exec-console proxy. The browser opens
/// `/ws/exec?env=<env>&id=<container>`; we connect upstream to the master
/// collector's `/ws/exec?id=...` and shuttle messages both ways: client text
/// frames (shell commands) go upstream, the collector's JSON output frames come
/// back down. Per-connection command senders are kept in `pending`, keyed by the
/// downstream socket id, so `on_message` can hand commands to the right task.
pub struct ExecWsCallback {
    app: Arc<AppCtx>,
    pending: Mutex<HashMap<i64, UnboundedSender<String>>>,
}

impl ExecWsCallback {
    pub fn new(app: Arc<AppCtx>) -> Self {
        Self {
            app,
            pending: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl MyWebSocketCallback for ExecWsCallback {
    async fn connected(
        &self,
        ws: Arc<MyWebSocket>,
        http_request: MyWebSocketHttpRequest,
        _disconnect_timeout: Duration,
    ) -> Result<(), WebSocketConnectedFail> {
        let user_id = http_request
            .get_headers()
            .get(crate::auth::SSL_USER_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let (env, container_id) = {
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
            (env, id)
        };

        // RBAC: same gate as the logs proxy.
        {
            let settings = self.app.settings_reader.get_settings().await;
            if !settings.is_env_allowed_for_user(&user_id, &env) {
                return Err(WebSocketConnectedFail {
                    reason: format!("env '{env}' is not accessible for user '{user_id}'"),
                    write_to_logs: true,
                });
            }
        }

        let (tx, rx) = unbounded_channel::<String>();
        self.pending.lock().unwrap().insert(ws.id, tx.clone());

        let app = self.app.clone();
        let ws_for_task = ws.clone();
        // `tx` is moved into the task purely to keep the channel open for the
        // task's lifetime (so `rx.recv()` never returns None while we run).
        tokio::spawn(async move {
            forward_exec(app, env, container_id, ws_for_task, rx, tx).await;
        });

        Ok(())
    }

    async fn disconnected(&self, ws: &MyWebSocket) {
        self.pending.lock().unwrap().remove(&ws.id);
    }

    async fn on_message(&self, ws: Arc<MyWebSocket>, message: WsMessage) {
        let command = match &message {
            WsMessage::Text(text) => text.to_string(),
            _ => return,
        };
        let sender = self.pending.lock().unwrap().get(&ws.id).cloned();
        if let Some(sender) = sender {
            let _ = sender.send(command);
        }
    }
}

async fn forward_exec(
    app: Arc<AppCtx>,
    env: String,
    container_id: String,
    ws: Arc<MyWebSocket>,
    mut rx: UnboundedReceiver<String>,
    _keep_tx: UnboundedSender<String>,
) {
    let settings = app.settings_reader.get_settings().await;
    let master_url = match settings.envs.get(&env) {
        Some(vm) => vm.url.clone(),
        None => {
            send_error(&ws, &format!("env '{env}' not configured")).await;
            ws.disconnect().await;
            return;
        }
    };

    let upstream_url = build_collector_ws_url(&master_url, &container_id);
    let mut upstream = match tokio_tungstenite::connect_async(&upstream_url).await {
        Ok((stream, _)) => stream,
        Err(err) => {
            send_error(
                &ws,
                &format!("failed to open exec WS to collector {master_url}: {err:?}"),
            )
            .await;
            ws.disconnect().await;
            return;
        }
    };

    let mut alive_check = tokio::time::interval(Duration::from_secs(2));
    alive_check.tick().await;
    let mut ping_tick = tokio::time::interval(Duration::from_secs(5));
    ping_tick.tick().await;

    loop {
        if !ws.is_connected() {
            break;
        }

        let mut send_cmd: Option<String> = None;
        let mut ping_upstream = false;

        tokio::select! {
            biased;
            _ = alive_check.tick() => {
                continue;
            }
            _ = ping_tick.tick() => {
                ws.send_message(std::iter::once(WsMessage::Ping(Vec::new().into()))).await;
                ping_upstream = true;
            }
            cmd = rx.recv() => {
                match cmd {
                    Some(c) => send_cmd = Some(c),
                    None => continue,
                }
            }
            msg = upstream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        ws.send_message(std::iter::once(WsMessage::Text(text.to_string().into())))
                            .await;
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        ws.send_message(std::iter::once(WsMessage::Binary(bytes.to_vec().into())))
                            .await;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if let Err(err) = upstream.send(Message::Pong(payload)).await {
                            println!("api ws-exec pong->upstream failed for {container_id}: {err:?}");
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        println!("api ws-exec upstream error for {container_id}: {err:?}");
                        break;
                    }
                }
            }
        }

        if let Some(c) = send_cmd {
            if let Err(err) = upstream.send(Message::Text(c.into())).await {
                println!("[api-ws-exec] -> command upstream failed id={container_id}: {err:?}");
                break;
            }
        }
        if ping_upstream {
            if let Err(err) = upstream.send(Message::Ping(Vec::new().into())).await {
                println!("[api-ws-exec] -> PING upstream failed id={container_id}: {err:?}");
                break;
            }
        }
    }

    ws.disconnect().await;
}

fn build_collector_ws_url(master_http_url: &str, container_id: &str) -> String {
    let scheme = if master_http_url.starts_with("https://") {
        "wss"
    } else {
        "ws"
    };
    let stripped = master_http_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    format!("{scheme}://{stripped}/ws/exec?id={container_id}")
}

async fn send_error(ws: &Arc<MyWebSocket>, msg: &str) {
    let escaped = serde_json::to_string(msg).unwrap_or_else(|_| "\"\"".to_string());
    let payload = format!(r#"{{"type":"error","text":{escaped}}}"#);
    ws.send_message(std::iter::once(WsMessage::Text(payload.into())))
        .await;
}
