use dioxus::prelude::*;

use crate::models::ContainerModel;
use crate::views::dockerscope::helpers::shorten_id;

#[component]
pub fn LabelsPanel(container: ContainerModel) -> Element {
    let mut rows: Vec<(String, String)> = Vec::new();
    rows.push((
        "container.id".to_string(),
        shorten_id(&container.id, 20).to_string(),
    ));
    rows.push(("image".to_string(), container.image.clone()));
    if let Some(state) = container.state.as_ref() {
        rows.push(("state".to_string(), state.clone()));
    }
    if let Some(status) = container.status.as_ref() {
        rows.push(("status".to_string(), status.clone()));
    }
    rows.push((
        "names".to_string(),
        container
            .names
            .iter()
            .map(|n| n.trim_start_matches('/').to_string())
            .collect::<Vec<_>>()
            .join(", "),
    ));
    if let Some(labels) = container.labels.as_ref() {
        for (k, v) in labels.iter() {
            rows.push((k.clone(), v.clone()));
        }
    }

    let count = rows.len();

    rsx! {
        div { class: "panel",
            div { class: "panel-head",
                h3 { "Metadata" }
                span { class: "count-pill", "{count} keys" }
            }
            table { class: "kv-table",
                tbody {
                    for (k, v) in rows.iter() {
                        LabelRow { k: k.clone(), v: v.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn LabelRow(k: String, v: String) -> Element {
    let dim = v.len() > 60;
    let v_class = if dim { "v dim" } else { "v" };
    rsx! {
        tr {
            td { class: "k", "{k}" }
            td { class: "{v_class}", "{v}" }
        }
    }
}
