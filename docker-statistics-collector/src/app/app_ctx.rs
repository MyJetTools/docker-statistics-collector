use std::sync::Arc;

use rust_extensions::AppStates;

use crate::settings::SettingsModel;

use super::{MetricsCache, PeersCache, ServicesCache};

pub const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &'static str = env!("CARGO_PKG_NAME");

pub struct AppContext {
    pub states: Arc<AppStates>,
    pub settings_model: Arc<SettingsModel>,
    pub cache: ServicesCache,

    pub metrics_cache: MetricsCache,
    pub peers_cache: PeersCache,
}

impl AppContext {
    pub fn new(settings_model: Arc<SettingsModel>) -> Self {
        AppContext {
            states: Arc::new(AppStates::create_initialized()),
            settings_model,
            cache: ServicesCache::new(),
            metrics_cache: MetricsCache::new(),
            peers_cache: PeersCache::new(),
        }
    }

    pub fn get_env_info(&self) -> String {
        match std::env::var("ENV_INFO") {
            Ok(value) => value,
            Err(_) => "NotSpecified".to_string(),
        }
    }
}
