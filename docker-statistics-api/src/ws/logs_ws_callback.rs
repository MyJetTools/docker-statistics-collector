use std::{sync::Arc, time::Duration};

use futures::{SinkExt, StreamExt};
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
        http_request: MyWebSocketHttpRequest,
        _disconnect_timeout: Duration,
    ) -> Result<(), WebSocketConnectedFail> {
        let user_id = http_request
            .get_headers()
            .get(crate::auth::SSL_USER_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

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

        // RBAC: drop the connection before opening anything if this user is
        // not allowed to see this env.
        {
            let settings = self.app.settings_reader.get_settings().await;
            if !settings.is_env_allowed_for_user(&user_id, &env) {
                return Err(WebSocketConnectedFail {
                    reason: format!(
                        "env '{env}' is not accessible for user '{user_id}'"
                    ),
                    write_to_logs: true,
                });
            }
        }

        let app = self.app.clone();
        let ws_for_task = ws.clone();
        tokio::spawn(async move {
            forward_logs(app, env, container_id, tail, ws_for_task).await;
        });

        Ok(())
    }

    async fn disconnected(&self, _ws: &MyWebSocket) {}

    async fn on_message(&self, _ws: Arc<MyWebSocket>, message: WsMessage) {
        // DEBUG: confirm whether the client's Pong (reply to our 5s Ping) — or
        // anything at all — actually reaches the API's read side. If we keep
        // sending PING but never see PONG here, the 60s idle close is explained.
        match &message {
            WsMessage::Pong(p) => println!("[api-ws-logs] <- PONG from client ({} bytes)", p.len()),
            WsMessage::Ping(p) => println!("[api-ws-logs] <- PING from client ({} bytes)", p.len()),
            WsMessage::Text(t) => println!("[api-ws-logs] <- TEXT from client: {:?}", t),
            WsMessage::Binary(b) => {
                println!("[api-ws-logs] <- BINARY from client ({} bytes)", b.len())
            }
            WsMessage::Close(_) => println!("[api-ws-logs] <- CLOSE from client"),
            WsMessage::Frame(_) => println!("[api-ws-logs] <- raw FRAME from client"),
        }
    }
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
    let mut upstream = match tokio_tungstenite::connect_async(&upstream_url).await {
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

    // NOTE: we deliberately do NOT call `.split()` on `upstream`. Split sinks
    // disable tokio-tungstenite's transparent Pong-on-Ping, which means the
    // collector would never see a Pong back and would eventually close us out
    // on its 60-second idle timeout. Keeping the stream whole and replying to
    // Pings explicitly inside this same loop keeps both directions alive.
    //
    // Timers:
    //  - alive_check (2s): cheap heartbeat into our own loop so a silent
    //    upstream doesn't keep an already-dead downstream connection open.
    //  - ping_tick (5s): WS Ping frame downstream so the browser doesn't
    //    auto-close the connection during quiet periods on the container.
    let mut alive_check = tokio::time::interval(std::time::Duration::from_secs(2));
    alive_check.tick().await;
    let mut ping_tick = tokio::time::interval(std::time::Duration::from_secs(5));
    ping_tick.tick().await;

    loop {
        if !ws.is_connected() {
            break;
        }
        tokio::select! {
            biased;
            _ = alive_check.tick() => {
                continue;
            }
            _ = ping_tick.tick() => {
                println!("[api-ws-logs] -> PING to client id={container_id}");
                ws.send_message(std::iter::once(WsMessage::Ping(Vec::new().into())))
                    .await;
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
                        // Reply manually — we own the writer in this loop too.
                        if let Err(err) = upstream.send(Message::Pong(payload)).await {
                            println!("api ws pong->upstream failed for {container_id}: {err:?}");
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
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
