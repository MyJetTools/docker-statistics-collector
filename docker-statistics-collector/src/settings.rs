use serde::Deserialize;

#[derive(my_settings_reader::SettingsModel, Clone, Deserialize)]
pub struct SettingsModel {
    pub vm_name: String,
    pub url: String,
    pub metrics_port: u16,
    pub disable_metics_collecting: Option<bool>,
    pub services_to_ignore: Vec<String>,
    pub api_version: String,
}

impl SettingsModel {
    pub fn ignore_service(&self, service: &str) -> bool {
        for service_from_settings in &self.services_to_ignore {
            if service_from_settings == service {
                return true;
            }
        }

        false
    }
}
