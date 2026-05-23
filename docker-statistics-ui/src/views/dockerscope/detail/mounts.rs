use dioxus::prelude::*;

use crate::models::VolumeHttpModel;
use crate::views::dockerscope::icons::icon_arrow_down;

#[component]
pub fn MountsPanel(volumes: Option<Vec<VolumeHttpModel>>) -> Element {
    let vols = volumes.unwrap_or_default();
    if vols.is_empty() {
        return rsx! {
            div { class: "panel",
                div { class: "panel-head", h3 { "Mounts" } }
                div { style: "padding:20px 4px; color:var(--text-muted); font-family:var(--mono); font-size:11.5px;",
                    "no mounts"
                }
            }
        };
    }

    let count = vols.len();

    rsx! {
        div { class: "panel",
            div { class: "panel-head",
                h3 { "Mounts" }
                span { class: "count-pill", "{count}" }
            }
            for v in vols.iter() {
                MountRow { mount: v.clone() }
            }
        }
    }
}

#[component]
fn MountRow(mount: VolumeHttpModel) -> Element {
    let kind = mount
        .mount_type
        .clone()
        .unwrap_or_else(|| "mount".to_string())
        .to_ascii_lowercase();
    let kind_class = format!("type {}", kind);
    let kind_upper = kind.to_ascii_uppercase();
    let src = mount
        .source
        .clone()
        .or_else(|| mount.name.clone())
        .unwrap_or_else(|| "?".to_string());
    let dst = mount.destination.clone().unwrap_or_else(|| "?".to_string());
    let rwo = if matches!(mount.rw, Some(false)) { "ro" } else { "rw" };

    rsx! {
        div { class: "mount-row",
            span { class: "{kind_class}", "{kind_upper}" }
            div { class: "paths",
                div { class: "src", "{src}" span { class: "rwo", "{rwo}" } }
                div { class: "dst",
                    span { class: "arrow-down", {icon_arrow_down()} }
                    "{dst}"
                }
            }
        }
    }
}
