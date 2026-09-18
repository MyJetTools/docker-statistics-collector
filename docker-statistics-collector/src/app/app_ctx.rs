use std::sync::Arc;

use rust_extensions::AppStates;

use crate::settings::SettingsModel;

use super::{DiskSizesCache, ExecPermission, LiveContainers};

pub const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &'static str = env!("CARGO_PKG_NAME");

pub struct AppContext {
    pub states: Arc<AppStates>,
    pub settings_model: Arc<SettingsModel>,

    /// On-demand reader of the local Docker host. The collector stores no
    /// container data of its own — every request is served from a fresh scan,
    /// and whoever asks (the API service) is the one that keeps history.
    pub live: LiveContainers,

    /// The one piece of state the collector keeps. Sizing a container costs Docker a
    /// walk of the storage layers, so a timer measures one at a time in the background
    /// and every payload carries the latest known values — see [`DiskSizesCache`].
    pub disk_sizes: DiskSizesCache,

    /// Time-limited unlock for the `exec_in_container` MCP tool. Starts disabled
    /// on every boot; a human opens it from the UI for a few minutes.
    pub exec_permission: ExecPermission,
}

impl AppContext {
    pub fn new(settings_model: Arc<SettingsModel>) -> Self {
        AppContext {
            states: Arc::new(AppStates::create_initialized()),
            live: LiveContainers::new(settings_model.clone()),
            disk_sizes: DiskSizesCache::new(),
            settings_model,
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
