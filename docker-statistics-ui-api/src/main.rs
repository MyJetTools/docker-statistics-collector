use std::sync::Arc;

use app::AppCtx;

mod app;
mod background;
mod http;
mod http_client;
mod models;
mod selected_vm;
mod settings_reader;

lazy_static::lazy_static! {
    pub static ref APP_CTX: Arc<AppCtx> = Arc::new(AppCtx::new());
}

#[tokio::main]
async fn main() {
    // Touch the lazy singleton — `AppCtx::new` registers and starts the metrics
    // poll timer (it spawns onto the current tokio runtime).
    let app = APP_CTX.clone();

    crate::http::start_up::setup_server(&app);

    app.app_states.wait_until_shutdown().await;
}
