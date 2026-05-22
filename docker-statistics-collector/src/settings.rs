use std::time::Duration;

use serde::Deserialize;

#[derive(my_settings_reader::SettingsModel, Clone, Deserialize)]
pub struct SettingsModel {
    pub docker_url: String,
    pub metrics_port: u16,
    pub disable_metics_collecting: Option<bool>,
    pub services_to_ignore: Option<Vec<String>>,
    pub peers: Option<Vec<String>>,
    pub peers_request_timeout_secs: Option<u64>,
    /// Path inside the collector container where the host `/proc` is visible.
    /// Used to read per-container open file descriptors and `nofile` limits.
    /// Defaults to `/host/proc` (the recommended bind-mount target).
    pub host_proc_path: Option<String>,
}

impl SettingsModel {
    pub fn ignore_service(&self, service: &str) -> bool {
        let Some(services_to_ignore) = self.services_to_ignore.as_ref() else {
            return false;
        };

        for service_from_settings in services_to_ignore {
            if service_from_settings == service {
                return true;
            }
        }

        false
    }

    pub fn peers_or_empty(&self) -> &[String] {
        match self.peers.as_ref() {
            Some(peers) => peers.as_slice(),
            None => &[],
        }
    }

    pub fn peers_request_timeout(&self) -> Duration {
        Duration::from_secs(self.peers_request_timeout_secs.unwrap_or(5))
    }

    pub fn host_proc_path(&self) -> &str {
        match self.host_proc_path.as_deref() {
            Some(path) => path,
            None => "/host/proc",
        }
    }
}
