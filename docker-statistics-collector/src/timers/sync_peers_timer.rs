use std::sync::Arc;

use flurl::FlUrl;
use rust_extensions::MyTimerTick;

use crate::app::AppContext;
use crate::http::controllers::containers_controller::contracts::ContainersHttpResponse;

pub struct SyncPeersTimer {
    app: Arc<AppContext>,
}

impl SyncPeersTimer {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl MyTimerTick for SyncPeersTimer {
    async fn tick(&self) {
        let peers = self.app.settings_model.peers_or_empty();
        if peers.is_empty() {
            return;
        }

        let timeout = self.app.settings_model.peers_request_timeout();
        let mut tasks = Vec::with_capacity(peers.len());

        for peer in peers {
            let peer_url = peer.clone();
            let app = self.app.clone();
            tasks.push(tokio::spawn(async move {
                sync_one_peer(app, peer_url, timeout).await
            }));
        }

        for task in tasks {
            let _ = task.await;
        }
    }
}

async fn sync_one_peer(
    app: Arc<AppContext>,
    peer_url: String,
    timeout: std::time::Duration,
) {
    let response = FlUrl::new(peer_url.as_str())
        .append_path_segment("api")
        .append_path_segment("containers")
        .append_path_segment("local")
        .set_timeout(timeout)
        .do_not_reuse_connection()
        .get()
        .await;

    let mut response = match response {
        Ok(r) => r,
        Err(err) => {
            app.peers_cache
                .record_error(&peer_url, format!("request failed: {:?}", err))
                .await;
            return;
        }
    };

    let status = response.get_status_code();
    if status != 200 {
        app.peers_cache
            .record_error(&peer_url, format!("status {}", status))
            .await;
        return;
    }

    let body = match response.get_body_as_slice().await {
        Ok(b) => b,
        Err(err) => {
            app.peers_cache
                .record_error(&peer_url, format!("body read failed: {:?}", err))
                .await;
            return;
        }
    };

    let parsed: ContainersHttpResponse = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(err) => {
            app.peers_cache
                .record_error(&peer_url, format!("parse failed: {}", err))
                .await;
            return;
        }
    };

    let containers = parsed
        .containers
        .into_iter()
        .map(|c| c.into_service_info())
        .collect();

    app.peers_cache
        .update(&peer_url, parsed.vm, containers)
        .await;
}
