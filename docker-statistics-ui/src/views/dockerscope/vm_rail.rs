use dioxus::prelude::*;

use crate::selected_vm::SelectedVm;
use crate::states::MainState;
use crate::views::dockerscope::helpers::*;
use crate::views::dockerscope::icons::*;

#[component]
pub fn VmRail() -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let cs_ra = main_state.read();

    let Some(vms) = cs_ra.vms_state.as_ref() else {
        return rsx! {
            aside { class: "vm-rail",
                div { class: "ds-loading", "loading vms…" }
            }
        };
    };

    let grouped = group_vms(vms);
    let active_vm = cs_ra.get_selected_vm_name();
    let all_selected = cs_ra.is_all_vms_selected();

    let prod_count = grouped.production.len();
    let stage_count = grouped.staging.len();
    let dev_count = grouped.dev.len();

    rsx! {
        aside { class: "vm-rail",
            VmGroupSection {
                label: "Production".to_string(),
                count: prod_count,
                items: grouped.production.into_iter().map(|(n, v)| (n.clone(), v.clone())).collect::<Vec<_>>(),
                active_vm: active_vm.clone(),
            }
            VmGroupSection {
                label: "Staging".to_string(),
                count: stage_count,
                items: grouped.staging.into_iter().map(|(n, v)| (n.clone(), v.clone())).collect::<Vec<_>>(),
                active_vm: active_vm.clone(),
            }
            VmGroupSection {
                label: "Dev / CI".to_string(),
                count: dev_count,
                items: grouped.dev.into_iter().map(|(n, v)| (n.clone(), v.clone())).collect::<Vec<_>>(),
                active_vm: active_vm.clone(),
            }
            // All VMs entry
            div {
                class: if all_selected { "vm-card active" } else { "vm-card" },
                style: "margin-top: 12px;",
                onclick: move |_| {
                    consume_context::<Signal<MainState>>()
                        .write()
                        .set_selected_vm(SelectedVm::All);
                },
                div { class: "ico", {icon_server()} }
                div { class: "body",
                    div { class: "name", "All VMs" }
                    div { class: "meta", span { class: "item", "aggregate" } }
                }
            }
        }
    }
}

#[component]
fn VmGroupSection(
    label: String,
    count: usize,
    items: Vec<(String, crate::models::VmModel)>,
    active_vm: Option<String>,
) -> Element {
    rsx! {
        div { class: "section-label",
            span { "{label}" }
            span { class: "pill", "{count}" }
        }
        for (name, vm) in items.iter() {
            VmCard {
                name: name.clone(),
                vm: vm.clone(),
                active: active_vm.as_deref() == Some(name.as_str()),
            }
        }
    }
}

#[component]
fn VmCard(name: String, vm: crate::models::VmModel, active: bool) -> Element {
    let status = vm_status(&vm);
    let heart_class = format!("heart {}", if status == "ok" { "" } else { status });
    let mem_pct = pct(vm.mem, vm.mem_limit) as i32;
    let cpu_pct = vm.cpu.round() as i32;
    let card_class = if active { "vm-card active" } else { "vm-card" };
    let name_for_click = name.clone();

    rsx! {
        div {
            class: "{card_class}",
            title: "{vm.api_url}",
            onclick: move |_| {
                consume_context::<Signal<MainState>>()
                    .write()
                    .set_selected_vm(crate::selected_vm::SelectedVm::SingleVm(name_for_click.clone()));
            },
            div { class: "ico",
                {icon_server()}
                span { class: "{heart_class}" }
            }
            div { class: "body",
                div { class: "name", "{name}" }
                div { class: "meta",
                    span { class: "item cpu", "{cpu_pct}%" }
                    span { class: "item mem", "{mem_pct}%" }
                }
            }
            div { class: "count", "{vm.containers_amount}" }
        }
    }
}
