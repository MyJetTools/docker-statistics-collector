use std::sync::Arc;

use flurl::FlUrl;
use rust_extensions::AppStates;
use tokio::sync::Mutex;

use crate::settings_reader::AppSettingsReader;

use super::DataCacheByEnv;

use crate::background::UpdateMetricsCacheTimer;
use rust_extensions::MyTimer;

/// Must exceed the master's worst case: its own live scan plus the peer fan-out
/// (`peers_request_timeout_secs`, default 30).
const MASTER_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);

pub struct AppCtx {
    pub data_cache_by_env: Mutex<DataCacheByEnv>,
    pub app_states: Arc<AppStates>,
    pub settings_reader: Arc<AppSettingsReader>,
}

impl AppCtx {
    pub fn new() -> Self {
        let app_states = Arc::new(AppStates::create_initialized());

        let mut timer_3s = MyTimer::new(std::time::Duration::from_secs(3));

        timer_3s.register_timer(
            "MetricsUpdate",
            std::sync::Arc::new(UpdateMetricsCacheTimer),
        );

        timer_3s.start(app_states.clone(), my_logger::LOGGER.clone());

        let settings_reader = Arc::new(AppSettingsReader::new());

        Self {
            data_cache_by_env: Mutex::new(DataCacheByEnv::new()),
            app_states,
            settings_reader,
        }
    }

    /// Request-handler path — runs inline on an HTTP request, not in a spawned task.
    /// The env and url come from the caller's query and the address from settings,
    /// so an unknown env, an unknown url or an address FlUrl cannot parse is the
    /// caller's error to be answered, not a reason to panic the request.
    pub async fn get_fl_url(&self, env: &str, url: &str) -> Result<FlUrl, String> {
        let settings = self.settings_reader.get_settings().await;

        let Some(env_settings) = settings.envs.get(env) else {
            return Err(format!("env {env} not found"));
        };

        if !env_settings.url.contains(url) {
            return Err(format!("url {url} not found in env {env}"));
        }

        let fl_url = FlUrl::try_new(env_settings.url.as_str()).map_err(|err| {
            format!("env {env}: cannot parse url {}: {:?}", env_settings.url, err)
        })?;

        Ok(self.configure_fl_url(fl_url))
    }

    /// Polling-timer path. It runs inside `tokio::spawn`, where a panic costs one
    /// tick of one env and the runtime logs it — so `FlUrl::new` is enough, and
    /// there is nothing to gain from threading an error out.
    pub fn create_fl_url(&self, url: &str) -> FlUrl {
        self.configure_fl_url(FlUrl::new(url))
    }

    fn configure_fl_url(&self, fl_url: FlUrl) -> FlUrl {
        fl_url
            // Explicit rather than FlUrl's 10s default: the master's answer is no longer
            // a cache read but its own live Docker scan plus a peer fan-out, and the
            // peer budget alone is 30s.
            .set_timeout(MASTER_REQUEST_TIMEOUT)
            .set_response_body_timeout(MASTER_REQUEST_TIMEOUT)
    }

}
