use dioxus::prelude::*;

use crate::{
    models::VmModel, selected_vm::SelectedVm, states::MainState, utils::format_mem, views::icons::*,
};

pub fn left_panel() -> Element {
    let env_name = use_signal(|| "".to_string());

    let mut main_state = consume_context::<Signal<MainState>>();

    let main_state_read_access = main_state.read();

    let selected_env = main_state_read_access.envs.get_selected_env().unwrap();

    let envs_options = if let Some(envs) = main_state_read_access.envs.items.try_unwrap_as_loaded()
    {
        envs.clone().into_iter().map(|env| {
            if env.as_str() == selected_env.as_str() {
                rsx! {
                    option { selected: true, {env.as_str()} }
                }
            } else {
                rsx! {
                    option { {env.as_str()} }
                }
            }
        })
    } else {
        return rsx! {
            option {}
        };
    };

    let env_name_value = env_name.read().clone();

    let mut all_vms = VmModel {
        api_url: "".to_string(),
        cpu: 0.0,
        mem: 0,
        mem_limit: 0,
        containers_amount: 0,
    };

    let vms_state = main_state_read_access.vms_state.as_ref();

    match vms_state {
        Some(vms) => {
            let items = vms.iter().map(|(vm, vm_model)| {
                let vm = vm.to_string();

                all_vms.cpu += vm_model.cpu;
                all_vms.mem += vm_model.mem;
                all_vms.containers_amount += vm_model.containers_amount;
                all_vms.mem_limit += vm_model.mem_limit;

                if main_state_read_access.is_single_vm_selected(&vm) {
                    rsx! {
                        div { class: "menu-item menu-active",
                            render_vm_menu_item {
                                name: vm.to_string(),
                                cpu: vm_model.cpu,
                                mem: vm_model.mem,
                                mem_limit: vm_model.mem_limit,
                                amount: vm_model.containers_amount,
                                url: vm_model.api_url.to_string(),
                            }
                        }
                    }
                } else {
                    rsx! {
                        div {
                            class: "menu-item",
                            onclick: move |_| {
                                main_state.write().set_selected_vm(SelectedVm::SingleVm(vm.to_string()));
                            },
                            render_vm_menu_item {
                                name: vm.to_string(),
                                cpu: vm_model.cpu,
                                mem: vm_model.mem,
                                mem_limit: vm_model.mem_limit,
                                amount: vm_model.containers_amount,
                                url: vm_model.api_url.to_string(),
                            }
                        }
                    }
                }
            });

            let mut items: Vec<_> = items.collect();

            items.push(rsx! {
                hr {}
            });

            let menu_active = if main_state_read_access.is_all_vms_selected() {
                "menu-active"
            } else {
                ""
            };

            items.push(rsx! {
                div {
                    class: "menu-item {menu_active}",
                    onclick: move |_| {
                        main_state.write().set_selected_vm(SelectedVm::All);
                    },
                    render_vm_menu_item {
                        name: "All VMs".to_string(),
                        cpu: all_vms.cpu,
                        mem: all_vms.mem,
                        mem_limit: all_vms.mem_limit,
                        amount: all_vms.containers_amount,
                        url: all_vms.api_url.to_string(),
                    }
                }
            });

            return rsx! {
                select {
                    class: "form-select",

                    oninput: |ctx| {
                        let value = ctx.value();
                        consume_context::<Signal<MainState>>()
                            .write()
                            .envs
                            .set_active_env(value.as_str());
                    },
                    {envs_options}
                }
                h1 { "Dockers" }
                h4 { id: "env-type", "{env_name_value}" }
                {items.into_iter()}
            };
        }
        None => {
            return rsx! { "Loading..." };
        }
    }
}

#[component]
fn render_vm_menu_item(
    name: String,
    cpu: f64,
    mem: i64,
    mem_limit: i64,
    amount: usize,
    url: String,
) -> Element {
    let mem = format_mem(mem);
    let mem_limit = format_mem(mem_limit);
    rsx! {
        table {
            tr { title: "{url}",
                td { server_icon {} }
                td {
                    div {
                        span { style: "font-size:12px", "{name}: ({amount})" }
                    }
                    div {
                        cpu_icon {}
                        span { font: "courier", style: "font-size:10px", ":{cpu:.3}  " }
                    }
                    div {
                        memory_icon {}
                        span { style: "font-size:10px", ":{mem}/{mem_limit}" }
                    }
                }
            }
        }
    }
}
