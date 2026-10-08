use crate::states::EditDiskTitleModel;

/// What the dialog itself holds: the text in the box. How the save is going is
/// not here — it lives on `EditDiskTitleModel`, which the dialog router writes.
pub struct EditDiskTitleState {
    pub value: String,
}

impl EditDiskTitleState {
    pub fn new(model: &EditDiskTitleModel) -> Self {
        Self {
            value: model.title.clone().unwrap_or_default(),
        }
    }

    pub fn set_value(&mut self, value: String) {
        self.value = value;
    }
}
