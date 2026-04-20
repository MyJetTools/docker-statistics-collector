use serde::Deserialize;

#[derive(my_settings_reader::SettingsModel, Clone, Deserialize)]
pub struct SettingsModel {
    pub docker_url: String,
    pub metrics_port: u16,
    pub disable_metics_collecting: Option<bool>,
    pub services_to_ignore: Option<Vec<String>>,
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
}
