#[derive(Clone, Debug)]
pub enum SelectedVm {
    All,
    SingleVm(String),
}

impl SelectedVm {
    pub fn from_string(value: String) -> Self {
        if value == "***All***" {
            return SelectedVm::All;
        }

        SelectedVm::SingleVm(value)
    }
}
