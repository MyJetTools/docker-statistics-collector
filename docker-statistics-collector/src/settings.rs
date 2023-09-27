use serde::Deserialize;

#[derive(my_settings_reader::SettingsModel, Deserialize)]
pub struct SettingsModel {
    pub vm_name: String,
    pub url: String,
}
