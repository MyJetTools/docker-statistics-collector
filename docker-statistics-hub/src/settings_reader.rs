use std::{collections::{BTreeMap, HashMap}, sync::Arc};

use my_settings_reader::SettingsReader;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingsModel {
    pub envs: BTreeMap<String, VmSettingsModel>,
    /// `user_id (== x-ssl-user header) -> group name`. Group `*` means
    /// "every env". A user that is not listed here gets nothing.
    pub users: Option<HashMap<String, String>>,
    /// `group name -> list of env names the group can see`.
    pub user_groups: Option<HashMap<String, Vec<String>>>,
    /// How many days the metrics of the services are kept on disk; older day files
    /// are deleted. [`DEFAULT_METRICS_DAYS_TO_KEEP`] when it is not set.
    pub metrics_days_to_keep: Option<u32>,
}

/// A week of metrics: some 2 GB for a hundred services polled every 3 seconds.
pub const DEFAULT_METRICS_DAYS_TO_KEEP: u32 = 7;

impl SettingsModel {
    /// Returns `(env, master_url)` per configured environment.
    /// One env = one federated master collector. The master fans out to its
    /// own configured peers in real time when answering UI requests.
    pub fn get_urls(&self) -> Vec<(String, String)> {
        self.envs
            .iter()
            .map(|(env, vm)| (env.clone(), vm.url.clone()))
            .collect()
    }

    /// Envs visible to the given user (from the `x-ssl-user` header).
    /// Rules:
    ///   * `users` is not configured at all → everyone sees all envs (dev mode)
    ///   * user is not in `users` → no envs
    ///   * user's group is `*` → all envs
    ///   * otherwise → intersection of `user_groups[group]` with configured envs
    pub fn get_envs_for_user(&self, user_id: &str) -> Vec<String> {
        let Some(users) = self.users.as_ref() else {
            return self.envs.keys().cloned().collect();
        };
        let Some(group) = users.get(user_id) else {
            return Vec::new();
        };
        if group == "*" {
            return self.envs.keys().cloned().collect();
        }
        let Some(groups) = self.user_groups.as_ref() else {
            return Vec::new();
        };
        let Some(allowed) = groups.get(group) else {
            return Vec::new();
        };
        allowed
            .iter()
            .filter(|e| self.envs.contains_key(e.as_str()))
            .cloned()
            .collect()
    }

    pub fn is_env_allowed_for_user(&self, user_id: &str, env: &str) -> bool {
        self.get_envs_for_user(user_id).iter().any(|e| e == env)
    }

    pub fn get_metrics_days_to_keep(&self) -> u32 {
        self.metrics_days_to_keep
            .unwrap_or(DEFAULT_METRICS_DAYS_TO_KEEP)
    }
}

pub struct AppSettingsReader {
    settings: SettingsReader<SettingsModel>,
}

impl AppSettingsReader {
    pub fn new() -> Self {
        Self {
            settings: SettingsReader::new("~/.docker-statistics-hub"),
        }
    }

    pub async fn get_settings(&self) -> Arc<SettingsModel> {
        self.settings.get_settings().await
    }

    pub async fn get_urls(&self) -> Vec<(String, String)> {
        let settings = self.settings.get_settings().await;
        settings.get_urls()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VmSettingsModel {
    pub url: String,
}
