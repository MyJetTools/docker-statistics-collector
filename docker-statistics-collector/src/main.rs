use std::{sync::Arc, time::Duration};

use rust_extensions::MyTimer;
use settings::SettingsModel;
use timers::{SyncContainersInfoTimer, SyncMetricsEndpointsTimer};

mod app;
mod host_mem;
mod http;
mod mcp;
mod peers_client;
mod proc_fd;
mod settings;
mod timers;
mod ws;
#[tokio::main]
async fn main() {
    match std::env::var("ENV_INFO") {
        Ok(v) if !v.trim().is_empty() => {}
        _ => {
            eprintln!(
                "FATAL: ENV_INFO environment variable is required and must be non-empty. \
                 It tags every container in federated responses and MUST be unique \
                 across peered collectors. Refusing to start."
            );
            std::process::exit(1);
        }
    }

    let settings = SettingsModel::read_from_file("~/.docker-statistics-collector".to_string())
        .await
        .unwrap();

    let app_ctx = app::AppContext::new(Arc::new(settings));

    let app_ctx = Arc::new(app_ctx);

    println!(
        "Instance name resolved to: {} (this value tags every container in federated responses; \
         it MUST be unique across peered collectors)",
        app_ctx.get_env_info()
    );

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

    http::start_http_server(&app_ctx).await;

    app_ctx.states.wait_until_shutdown().await;
}
