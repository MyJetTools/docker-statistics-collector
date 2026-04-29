use std::{sync::Arc, time::Duration};

use rust_extensions::MyTimer;
use settings::SettingsModel;
use timers::{SyncContainersInfoTimer, SyncMetricsEndpointsTimer, SyncPeersTimer};

mod app;
mod http;
mod mcp;
mod settings;
mod timers;
#[tokio::main]
async fn main() {
    let settings = SettingsModel::read_from_file("~/.docker-statistics-collector".to_string())
        .await
        .unwrap();

    let app_ctx = app::AppContext::new(Arc::new(settings));

    let app_ctx = Arc::new(app_ctx);

    let mut timer_5s =
        MyTimer::new_with_execute_timeout(Duration::from_secs(5), Duration::from_secs(60 * 5));

    timer_5s.register_timer(
        "Containers reader",
        Arc::new(SyncContainersInfoTimer::new(app_ctx.clone())),
    );

    timer_5s.register_timer(
        "Sync metrics",
        Arc::new(SyncMetricsEndpointsTimer::new(app_ctx.clone())),
    );

    timer_5s.start(app_ctx.states.clone(), my_logger::LOGGER.clone());

    if !app_ctx.settings_model.peers_or_empty().is_empty() {
        let peers_interval = app_ctx.settings_model.peers_sync_interval();
        let mut peers_timer =
            MyTimer::new_with_execute_timeout(peers_interval, Duration::from_secs(60));
        peers_timer.register_timer(
            "Sync peers",
            Arc::new(SyncPeersTimer::new(app_ctx.clone())),
        );
        peers_timer.start(app_ctx.states.clone(), my_logger::LOGGER.clone());
    }

    http::start_http_server(&app_ctx).await;

    app_ctx.states.wait_until_shutdown().await;
}
