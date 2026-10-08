use dioxus::prelude::*;

use crate::states::{DialogState, EditDiskTitleModel, MainState};

/// Save the title typed into the dialog. Run by the dialog router, not by the
/// dialog: once the api has it the rail shows it at once and the dialog closes;
/// refused, the dialog stays open with the reason.
pub fn save_disk_title(
    mut dialog: Signal<DialogState>,
    mut main_state: Signal<MainState>,
    model: EditDiskTitleModel,
    title: String,
) {
    // Enter on the box and the save button both end up here.
    if model.saving {
        return;
    }

    dialog
        .write()
        .disk_title_save_started(model.vm.as_str(), model.mount_point.as_str());

    spawn(async move {
        let result = crate::api::set_disk_title(
            model.env.as_str(),
            model.vm.as_str(),
            model.mount_point.as_str(),
            title.as_str(),
        )
        .await;

        match result {
            Ok(saved) => {
                main_state.write().set_disk_title(
                    model.vm.as_str(),
                    model.mount_point.as_str(),
                    saved,
                );
                dialog
                    .write()
                    .disk_title_saved(model.vm.as_str(), model.mount_point.as_str());
            }
            Err(err) => {
                dialog.write().disk_title_save_failed(
                    model.vm.as_str(),
                    model.mount_point.as_str(),
                    err.to_string(),
                );
            }
        }
    });
}
