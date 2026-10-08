use std::rc::Rc;

pub enum DialogType {
    /*
    ShowSecret(String),
    AddSecret,

    EditSecret(String),
    DeleteSecret(String),

    AddTemplate,
    AddTemplateFromOtherTemplate { env: String, name: String },
    EditTemplate { env: String, name: String },
    DeleteTemplate { env: String, name: String }, */
    ShowLogs {
        env: Rc<String>,
        url: String,
        container_id: String,
    },
    ShowProcesses {
        env: Rc<String>,
        url: String,
        container_id: String,
    },
    ShowExec {
        env: Rc<String>,
        url: String,
        container_id: String,
    },
    EditDiskTitle(EditDiskTitleModel),
    /*
    SecretUsage(String),
    SecretUsageBySecret(String),
     */
}

impl DialogType {
    /// Class of the modal. The viewers want all the width they can get; a
    /// one-field form does not.
    pub fn modal_class(&self) -> &'static str {
        match self {
            DialogType::EditDiskTitle(_) => "ds-modal ds-modal-sm",
            _ => "ds-modal",
        }
    }
}

/// The host disk whose title is being edited, and how the save is going. The
/// save is run by the dialog router rather than by the dialog, so this is where
/// it reports back.
#[derive(Clone, PartialEq)]
pub struct EditDiskTitleModel {
    pub env: Rc<String>,
    pub vm: String,
    /// The disk's identity on its VM — titles are stored by mount point.
    pub mount_point: String,
    pub device: String,
    /// The title the disk has now, if any.
    pub title: Option<String>,
    /// A save is on its way to the api.
    pub saving: bool,
    /// Why the last save was refused.
    pub error: Option<String>,
}

pub enum DialogState {
    Hidden,
    Shown {
        header: String,
        dialog_type: DialogType,
    },
}

impl DialogState {
    pub fn show_dialog(&mut self, header: String, dialog_type: DialogType) {
        *self = Self::Shown {
            header,
            dialog_type,
        };
    }

    pub fn hide_dialog(&mut self) {
        *self = Self::Hidden;
    }

    pub fn as_ref(&self) -> &Self {
        self
    }

    pub fn edit_disk_title(&mut self, model: EditDiskTitleModel) {
        self.show_dialog(
            format!("Disk title · {} {}", model.vm, model.mount_point),
            DialogType::EditDiskTitle(model),
        );
    }

    pub fn disk_title_save_started(&mut self, vm: &str, mount_point: &str) {
        if let Some(model) = self.edited_disk_mut(vm, mount_point) {
            model.saving = true;
            model.error = None;
        }
    }

    /// The save was refused: the dialog stays open and shows why.
    pub fn disk_title_save_failed(&mut self, vm: &str, mount_point: &str, error: String) {
        if let Some(model) = self.edited_disk_mut(vm, mount_point) {
            model.saving = false;
            model.error = Some(error);
        }
    }

    pub fn disk_title_saved(&mut self, vm: &str, mount_point: &str) {
        if self.edited_disk_mut(vm, mount_point).is_some() {
            self.hide_dialog();
        }
    }

    /// The edit dialog of exactly this disk, if it is the one on screen. A save
    /// answers after an await, and by then the dialog may have been closed or
    /// another one opened — the answer must not land on that one.
    fn edited_disk_mut(&mut self, vm: &str, mount_point: &str) -> Option<&mut EditDiskTitleModel> {
        match self {
            Self::Shown {
                dialog_type: DialogType::EditDiskTitle(model),
                ..
            } if model.vm == vm && model.mount_point == mount_point => Some(model),
            _ => None,
        }
    }
}
