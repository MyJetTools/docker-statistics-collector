use serde::Deserialize;

#[derive(my_settings_reader::SettingsModel, Clone, Deserialize)]
pub struct SettingsModel {
    pub vm_name: String,
    pub url: String,
    pub metrics_port: u16,
}
