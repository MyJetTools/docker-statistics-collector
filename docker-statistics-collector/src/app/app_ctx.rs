use std::sync::Arc;

use rust_extensions::AppStates;

use crate::settings::SettingsModel;

use super::{MetricsCache, ServicesCache};

pub const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &'static str = env!("CARGO_PKG_NAME");

pub struct AppContext {
    pub states: Arc<AppStates>,
    pub settings_model: Arc<SettingsModel>,
    pub cache: ServicesCache,

    pub metrics_cache: MetricsCache,
}

impl AppContext {
    pub fn new(settings_model: Arc<SettingsModel>) -> Self {
        AppContext {
            states: Arc::new(AppStates::create_initialized()),
            settings_model,
            cache: ServicesCache::new(),
            metrics_cache: MetricsCache::new(),
        }
    }
}
