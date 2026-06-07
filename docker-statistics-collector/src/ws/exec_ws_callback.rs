use std::{sync::Arc, time::Duration};

use my_http_server::web_sockets::{
    MyWebSocket, MyWebSocketCallback, MyWebSocketHttpRequest, WebSocketConnectedFail, WsMessage,
};

use crate::app::AppContext;
use crate::peers_client::{fanout_exec, RouteExecResult};

/// Interactive "exec console" WebSocket. The client opens
/// `ws://host/ws/exec?id=<container>` and sends one shell command per text
/// message; for each command we run it (`sh -c "<command>"`, auto-routed to the
/// owning instance or peer via `fanout_exec`) and stream the result back as JSON
/// messages. Commands are independent — there is no persistent shell session, so
/// state like the working directory does not carry between commands.
pub struct ExecWsCallback {
    app: Arc<AppContext>,
}

impl ExecWsCallback {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl MyWebSocketCallback for ExecWsCallback {
    async fn connected(
        &self,
        ws: Arc<MyWebSocket>,
        _http_request: MyWebSocketHttpRequest,
        _disconnect_timeout: Duration,
    ) -> Result<(), WebSocketConnectedFail> {
        // Validate the container id is present up front.
        let container_id = match ws.get_query_string() {
            Some(q) => match q.get_required("id").and_then(|v| v.as_string()) {
                Ok(s) => s,
                Err(_) => {
                    return Err(WebSocketConnectedFail {
                        reason: "missing 'id' query param".to_string(),
                        write_to_logs: false,
                    });
                }
            },
            None => {
                return Err(WebSocketConnectedFail {
                    reason: "missing query string".to_string(),
                    write_to_logs: false,
                });
            }
        };

        send_json(
            &ws,
            &serde_json::json!({
                "type": "info",
                "text": format!(
                    "Connected to {}. Each command runs as `sh -c`; state does not persist between commands.",
                    container_id
                ),
            }),
        )
        .await;

        // Keepalive — without periodic traffic the middleware closes the idle
        // console between commands.
        let ws_ping = ws.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(5));
            tick.tick().await; // skip the immediate first tick
            loop {
                tick.tick().await;
                if !ws_ping.is_connected() {
                    break;
                }
                ws_ping
                    .send_message(std::iter::once(WsMessage::Ping(Vec::new().into())))
                    .await;
            }
        });

        Ok(())
    }

    async fn disconnected(&self, _ws: &MyWebSocket) {}

    async fn on_message(&self, ws: Arc<MyWebSocket>, message: WsMessage) {
        let command = match &message {
            WsMessage::Text(text) => text.to_string(),
            _ => return,
        };
        let command = command.trim().to_string();
        if command.is_empty() {
            return;
        }

        let container_id = match ws.get_query_string() {
            Some(q) => match q.get_required("id").and_then(|v| v.as_string()) {
                Ok(s) => s,
                Err(_) => return,
            },
            None => return,
        };

        // Echo the command so the terminal shows what was run.
        send_json(
            &ws,
            &serde_json::json!({ "type": "input", "text": command }),
        )
        .await;

        match fanout_exec(&self.app, &container_id, &command).await {
            RouteExecResult::Ok(result) => {
                if !result.output.is_empty() {
                    send_json(
                        &ws,
                        &serde_json::json!({ "type": "output", "text": result.output }),
                    )
                    .await;
                }
                send_json(
                    &ws,
                    &serde_json::json!({ "type": "exit", "code": result.exit_code }),
                )
                .await;
            }
            RouteExecResult::NotFound => {
                send_json(
                    &ws,
                    &serde_json::json!({
                        "type": "error",
                        "text": format!("container {} not found on this instance or any peer", container_id),
                    }),
                )
                .await;
            }
            RouteExecResult::PeerError(err) => {
                send_json(
                    &ws,
                    &serde_json::json!({ "type": "error", "text": err }),
                )
                .await;
            }
        }
    }
}

async fn send_json(ws: &MyWebSocket, value: &serde_json::Value) {
    let payload = value.to_string();
    ws.send_message(std::iter::once(WsMessage::Text(payload.into())))
        .await;
}
