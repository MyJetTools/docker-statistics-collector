#[derive(Clone, Debug)]
pub enum SelectedVm {
    All,
    SingleVm(String),
}

impl SelectedVm {
    pub fn is_all(&self) -> bool {
        match self {
            SelectedVm::All => true,
            _ => false,
        }
    }

    pub fn is_single_selected_with_name(&self, name: &str) -> bool {
        match self {
            SelectedVm::SingleVm(value) => {
                return value == name;
            }
            _ => false,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            SelectedVm::All => "***All***".to_string(),
            SelectedVm::SingleVm(value) => value.to_string(),
        }
    }

    #[cfg(feature = "server")]
    pub fn from_string(value: String) -> Self {
        if value == "***All***" {
            return SelectedVm::All;
        }

        SelectedVm::SingleVm(value)
    }
}
