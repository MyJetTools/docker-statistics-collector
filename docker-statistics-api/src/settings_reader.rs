use std::{collections::{BTreeMap, HashMap}, sync::Arc};

use my_settings_reader::SettingsReader;
use my_ssh::ssh_settings::SshPrivateKeySettingsModel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingsModel {
    pub envs: BTreeMap<String, VmSettingsModel>,
    pub ssh_private_keys: Option<HashMap<String, SshPrivateKeySettingsModel>>,
    pub prompt_pass_phrase: Option<bool>,
}

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

    pub fn get_envs(&self) -> Vec<String> {
        self.envs.keys().cloned().collect()
    }
}

pub struct AppSettingsReader {
    settings: SettingsReader<SettingsModel>,
}

impl AppSettingsReader {
    pub fn new() -> Self {
        Self {
            settings: SettingsReader::new("~/.docker-statistics-api"),
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
