use dioxus::prelude::*;

use crate::models::ContainerModel;
use crate::views::dockerscope::helpers::pct;

#[component]
pub fn StatLine(container: ContainerModel) -> Element {
    let cpu = container.cpu.usage.unwrap_or(0.0);
    let mem_used = container.mem.usage.unwrap_or(0);
    let mem_limit = container.mem.limit.unwrap_or(0);
    let mem_pct = pct(mem_used, mem_limit) as i32;
    let files_open = container.files.open.unwrap_or(0);
    let files_limit = container.files.limit.unwrap_or(0);
    let files_pct = pct(files_open, files_limit) as i32;

    rsx! {
        div { class: "statline",
            Stat { k: "CPU usage", v: format!("{:.2}", cpu), u: "%".to_string() }
            Stat { k: "Memory %",   v: format!("{}",     mem_pct), u: "%".to_string() }
            Stat { k: "Open FDs",   v: format!("{}",     files_open), u: format!("/ {}", files_limit) }
            Stat { k: "FD %",       v: format!("{}",     files_pct), u: "%".to_string() }
        }
    }
}

#[component]
fn Stat(k: String, v: String, u: String) -> Element {
    rsx! {
        div { class: "stat",
            div { class: "k", "{k}" }
            div { class: "v",
                "{v}"
                span { class: "u", "{u}" }
            }
        }
    }
}
