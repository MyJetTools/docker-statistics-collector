use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use my_http_server::web_sockets::{
    MyWebSocket, MyWebSocketCallback, MyWebSocketHttpRequest, WebSocketConnectedFail, WsMessage,
};
use tokio_tungstenite::tungstenite::Message;

use crate::app::AppContext;

use super::LogFrameParser;

pub struct LogsWsCallback {
    app: Arc<AppContext>,
}

impl LogsWsCallback {
    pub fn new(app: Arc<AppContext>) -> Self {
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
        let (container_id, tail) = {
            let query = match ws.get_query_string() {
                Some(q) => q,
                None => {
                    return Err(WebSocketConnectedFail {
                        reason: "missing query string".to_string(),
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
                .and_then(|v| v.from_str::<u32>().ok())
                .unwrap_or(200);
            (id, tail)
        };

        let app = self.app.clone();
        let ws_for_task = ws.clone();
        tokio::spawn(async move {
            println!("[col-ws-logs] connected id={container_id} tail={tail}");
            if is_local(&app, &container_id).await {
                println!("[col-ws-logs] id={container_id} is LOCAL — streaming from docker {}",
                    app.settings_model.docker_url);
                stream_local_logs_to_ws(
                    &app.settings_model.docker_url,
                    &container_id,
                    tail,
                    ws_for_task,
                )
                .await;
                return;
            }
            println!("[col-ws-logs] id={container_id} is NOT local — will fan-out to peers");

            // Container is not on this docker host — try peers in order. The
            // first peer that owns it streams the lines back through us to the
            // client. Peers that don't own it respond with one error JSON and
            // close immediately, so we move on to the next.
            let peers: Vec<String> = app.settings_model.peers_or_empty().to_vec();
            if peers.is_empty() {
                send_error_line(&ws_for_task, "container not local and no peers configured")
                    .await;
                ws_for_task.disconnect().await;
                return;
            }

            for peer in peers {
                if !ws_for_task.is_connected() {
                    return;
                }
                if try_peer_and_forward(&peer, &container_id, tail, ws_for_task.clone()).await {
                    return;
                }
            }

            send_error_line(&ws_for_task, "container not found on any peer in this env").await;
            ws_for_task.disconnect().await;
        });

        Ok(())
    }

    async fn disconnected(&self, _ws: &MyWebSocket) {
        // Streaming task notices `ws.is_connected() == false` on its next chunk
        // and exits; nothing extra to clean up here.
    }

    async fn on_message(&self, _ws: Arc<MyWebSocket>, _message: WsMessage) {
        // We don't read anything from the client — UI is a pure consumer.
    }
}

async fn is_local(app: &AppContext, container_id: &str) -> bool {
    app.cache
        .get_snapshot()
        .await
        .iter()
        .any(|c| c.id == container_id)
}

async fn stream_local_logs_to_ws(
    docker_url: &str,
    container_id: &str,
    tail: u32,
    ws: Arc<MyWebSocket>,
) {
    let stream = docker_sdk::sdk::get_container_logs_stream(docker_url, container_id, tail).await;
    let mut stream = match stream {
        Ok(s) => {
            println!("[col-ws-logs] docker stream opened for id={container_id}");
            s
        }
        Err(err) => {
            eprintln!("[col-ws-logs] FAILED to open docker stream for id={container_id}: {err:?}");
            let payload = format!(r#"{{"error":"failed to open docker stream: {err:?}"}}"#);
            let _ = ws
                .send_message(std::iter::once(WsMessage::Text(payload.into())))
                .await;
            ws.disconnect().await;
            return;
        }
    };

    let mut parser = LogFrameParser::new();
    let mut sent = 0u64;
    let mut chunks = 0u64;

    // Heartbeat — without it the WS middleware closes idle connections on a
    // quiet container. 5s is well under any reasonable idle timeout.
    let mut ping_tick = tokio::time::interval(std::time::Duration::from_secs(5));
    ping_tick.tick().await; // skip immediate first tick

    loop {
        if !ws.is_connected() {
            break;
        }

        let chunk = tokio::select! {
            _ = ping_tick.tick() => {
                ws.send_message(std::iter::once(WsMessage::Ping(Vec::new().into())))
                    .await;
                continue;
            }
            c = stream.get_next_chunk() => c,
        };
        let chunk = match chunk {
            Ok(Some(bytes)) => bytes,
            Ok(None) => {
                println!("[col-ws-logs] docker stream ENDED for id={container_id} after {chunks} chunks / {sent} lines");
                break;
            }
            Err(err) => {
                eprintln!("[col-ws-logs] docker stream error id={container_id}: {err:?}");
                break;
            }
        };

        chunks += 1;
        if chunks <= 3 {
            println!("[col-ws-logs] id={container_id} chunk #{chunks} ({} bytes)", chunk.len());
        }

        parser.feed(&chunk);
        let lines = parser.take_lines();
        if lines.is_empty() {
            continue;
        }

        for (tp, line) in lines {
            let escaped = serde_json::to_string(&line)
                .unwrap_or_else(|_| "\"\"".to_string());
            let payload = format!(r#"{{"tp":{tp},"line":{escaped}}}"#);
            ws.send_message(std::iter::once(WsMessage::Text(payload.into())))
                .await;
            sent += 1;
            if sent <= 3 {
                println!("[col-ws-logs] id={container_id} sent line #{sent}: tp={tp} {:?}",
                    if line.len() > 120 { &line[..120] } else { &line });
            }
        }
    }

    println!("[col-ws-logs] disconnecting id={container_id} (sent {sent} lines)");
    ws.disconnect().await;
}

/// Open a WS to a peer collector and try forwarding its log frames. If the
/// first message back is an error JSON (peer doesn't own the container), we
/// close that peer cleanly and return `false` so the caller can try the next.
/// Returns `true` if the peer owned the container — caller should stop here.
async fn try_peer_and_forward(
    peer_http_url: &str,
    container_id: &str,
    tail: u32,
    client_ws: Arc<MyWebSocket>,
) -> bool {
    let ws_url = build_collector_ws_url(peer_http_url, container_id, Some(tail));
    let stream = match tokio_tungstenite::connect_async(&ws_url).await {
        Ok((s, _)) => s,
        Err(err) => {
            println!("peer logs WS connect failed for {peer_http_url}: {err:?}");
            return false;
        }
    };

    let (_write, mut read) = stream.split();

    let mut owned_by_this_peer = false;
    while let Some(msg) = read.next().await {
        if !client_ws.is_connected() {
            return true; // client gone; no need to keep trying peers
        }
        match msg {
            Ok(Message::Text(text)) => {
                let s = text.to_string();
                if !owned_by_this_peer && looks_like_error_payload(&s) {
                    // Peer doesn't own it — stop reading this peer and try the next.
                    return false;
                }
                owned_by_this_peer = true;
                client_ws
                    .send_message(std::iter::once(WsMessage::Text(s.into())))
                    .await;
            }
            Ok(Message::Binary(bytes)) => {
                owned_by_this_peer = true;
                client_ws
                    .send_message(std::iter::once(WsMessage::Binary(bytes.to_vec().into())))
                    .await;
            }
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(_) => {}
        }
    }

    owned_by_this_peer
}

fn build_collector_ws_url(http_url: &str, container_id: &str, tail: Option<u32>) -> String {
    let scheme = if http_url.starts_with("https://") {
        "wss"
    } else {
        "ws"
    };
    let host = http_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    let mut url = format!("{scheme}://{host}/ws/logs?id={container_id}");
    if let Some(t) = tail {
        url.push_str(&format!("&tail={t}"));
    }
    url
}

fn looks_like_error_payload(s: &str) -> bool {
    // We send error frames as `{"error":"..."}` — cheap prefix check is enough
    // and avoids parsing a JSON value on every text message just for routing.
    s.trim_start().starts_with(r#"{"error""#)
}

async fn send_error_line(ws: &Arc<MyWebSocket>, msg: &str) {
    let escaped = serde_json::to_string(msg).unwrap_or_else(|_| "\"\"".to_string());
    let payload = format!(r#"{{"error":{escaped}}}"#);
    ws.send_message(std::iter::once(WsMessage::Text(payload.into())))
        .await;
}
