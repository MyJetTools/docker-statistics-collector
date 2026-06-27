use std::io::Read;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use dioxus::prelude::*;
use flate2::read::GzDecoder;

use crate::models::ContainerModel;

/// Docker label release-mcp stamps onto a container, holding the
/// gzip+base64-encoded docker-compose.yaml that produced it.
const COMPOSE_LABEL: &str = "com.release-mcp.compose-yaml";

/// Decoded docker-compose.yaml panel, shown right under the log tail. It only
/// renders when the container actually carries the `com.release-mcp.compose-yaml`
/// label (and it decodes to something non-empty) — for any other container the
/// whole widget collapses to nothing.
#[component]
pub fn ComposePanel(container: ContainerModel) -> Element {
    let Some(raw) = container
        .labels
        .as_ref()
        .and_then(|labels| labels.get(COMPOSE_LABEL))
    else {
        return rsx! {};
    };

    let yaml = decode_compose_label(raw);
    if yaml.trim().is_empty() {
        return rsx! {};
    }

    let line_count = yaml.lines().count();

    rsx! {
        div { class: "panel",
            div { class: "panel-head",
                h3 { "docker-compose.yaml" }
                span { class: "count-pill", "{line_count} lines" }
            }
            pre { class: "log-mini log-mini-tall compose-pre", "{yaml}" }
        }
    }
}

/// base64 → gzip → text. If anything fails to inflate we fall back to the raw
/// bytes (uncompressed label, foreign format, …) so the content stays readable
/// instead of disappearing — same logic as the collector's compose_blob.rs.
fn decode_compose_label(value: &str) -> String {
    if let Ok(bytes) = BASE64.decode(value.trim()) {
        let mut out = String::new();
        if GzDecoder::new(&bytes[..]).read_to_string(&mut out).is_ok() {
            return out; // inflated YAML
        }
        if let Ok(text) = String::from_utf8(bytes) {
            return text; // base64 of raw text
        }
    }
    value.to_string() // not base64 → already raw text
}
