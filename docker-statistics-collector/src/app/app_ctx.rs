use std::sync::Arc;

use rust_extensions::AppStates;

use crate::settings::SettingsModel;

use super::{ExecPermission, MetricsCache, ServicesCache};

pub const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &'static str = env!("CARGO_PKG_NAME");

pub struct AppContext {
    pub states: Arc<AppStates>,
    pub settings_model: Arc<SettingsModel>,
    pub cache: ServicesCache,

    pub metrics_cache: MetricsCache,

    /// Time-limited unlock for the `exec_in_container` MCP tool. Starts disabled
    /// on every boot; a human opens it from the UI for a few minutes.
    pub exec_permission: ExecPermission,
}

impl AppContext {
    pub fn new(settings_model: Arc<SettingsModel>) -> Self {
        AppContext {
            states: Arc::new(AppStates::create_initialized()),
            settings_model,
            cache: ServicesCache::new(),
            metrics_cache: MetricsCache::new(),
            exec_permission: ExecPermission::new(),
        }
    }

    /// Instance name used to tag containers in federated responses. Comes from
    /// the `ENV_INFO` environment variable, which is verified to be set at
    /// startup in `main`.
    pub fn get_env_info(&self) -> String {
        std::env::var("ENV_INFO")
            .expect("ENV_INFO must be set (checked at startup in main)")
    }
}
