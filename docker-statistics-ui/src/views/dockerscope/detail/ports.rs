use dioxus::prelude::*;

use crate::models::PortHttpModel;
use crate::views::dockerscope::icons::icon_ext;

#[component]
pub fn PortsPanel(ports: Option<Vec<PortHttpModel>>) -> Element {
    let ports = ports.unwrap_or_default();
    if ports.is_empty() {
        return rsx! {
            div { class: "panel",
                div { class: "panel-head", h3 { "Ports" } }
                div { style: "padding:20px 4px; color:var(--text-muted); font-family:var(--mono); font-size:11.5px;",
                    "no published ports"
                }
            }
        };
    }

    let count = ports.len();

    rsx! {
        div { class: "panel",
            div { class: "panel-head",
                h3 { "Ports" }
                span { class: "count-pill", "{count}" }
            }
            for p in ports.iter() {
                PortRow { port: p.clone() }
            }
        }
    }
}

#[component]
fn PortRow(port: PortHttpModel) -> Element {
    let proto_lower = port.port_type.to_ascii_lowercase();
    let proto_class = format!("proto {}", proto_lower);
    let proto_upper = port.port_type.to_ascii_uppercase();
    let host_text = match port.public_port {
        Some(p) => format!(
            "{}:{}",
            port.ip.clone().unwrap_or_else(|| "0.0.0.0".to_string()),
            p
        ),
        None => "—".to_string(),
    };

    rsx! {
        div { class: "port-row",
            span { class: "{proto_class}", "{proto_upper}" }
            span { class: "mapping",
                span { class: "host", "{host_text}" }
                span { class: "arrow", "→" }
                span { class: "container", ":{port.private_port}" }
            }
            span { class: "ext", "↗ host" }
            span { class: "link", title: "open in browser", {icon_ext()} }
        }
    }
}
