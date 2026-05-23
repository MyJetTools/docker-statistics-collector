use core::panic;
use std::sync::Arc;

use flurl::FlUrl;
use rust_extensions::AppStates;
use tokio::sync::Mutex;

use crate::settings_reader::AppSettingsReader;

use super::{DataCacheByEnv, SshPrivateKeyResolver};

use crate::background::UpdateMetricsCacheTimer;
use rust_extensions::MyTimer;

pub struct AppCtx {
    pub data_cache_by_env: Mutex<DataCacheByEnv>,
    pub app_states: Arc<AppStates>,
    pub settings_reader: Arc<AppSettingsReader>,
    pub ssh_private_key_resolver: Arc<SshPrivateKeyResolver>,
}

impl AppCtx {
    pub fn new() -> Self {
        let app_states = Arc::new(AppStates::create_initialized());

        let mut timer_5s = MyTimer::new(std::time::Duration::from_secs(3));

        timer_5s.register_timer(
            "MetricsUpdate",
            std::sync::Arc::new(UpdateMetricsCacheTimer),
        );

        timer_5s.start(app_states.clone(), my_logger::LOGGER.clone());

        let settings_reader = Arc::new(AppSettingsReader::new());

        Self {
            ssh_private_key_resolver: SshPrivateKeyResolver::new(settings_reader.clone()).into(),
            data_cache_by_env: Mutex::new(DataCacheByEnv::new()),
            app_states,
            settings_reader,
        }
    }

    pub async fn get_fl_url(&self, env: &str, url: &str) -> FlUrl {
        let settings = self.settings_reader.get_settings().await;

        let env_settings = settings.envs.get(env);

        if env_settings.is_none() {
            panic!("Env {env} not found");
        }

        let env_settings = env_settings.unwrap();

        if env_settings.url.contains(url) {
            return self.create_fl_url(env_settings.url.as_str());
        }

        panic!("Url {url} not found in env {env}");
    }

    pub fn create_fl_url(&self, url: &str) -> FlUrl {
        FlUrl::new(url).set_ssh_security_credentials_resolver(self.ssh_private_key_resolver.clone())
    }
}
