use std::time::Duration;

use serde::Deserialize;

#[derive(my_settings_reader::SettingsModel, Clone, Deserialize)]
pub struct SettingsModel {
    pub docker_url: String,
    pub metrics_port: u16,
    pub disable_metics_collecting: Option<bool>,
    pub services_to_ignore: Option<Vec<String>>,
    pub peers: Option<Vec<String>>,
    pub peers_sync_interval_secs: Option<u64>,
    pub peers_request_timeout_secs: Option<u64>,
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

    pub fn peers_sync_interval(&self) -> Duration {
        Duration::from_secs(self.peers_sync_interval_secs.unwrap_or(5))
    }

    pub fn peers_request_timeout(&self) -> Duration {
        Duration::from_secs(self.peers_request_timeout_secs.unwrap_or(5))
    }
}
