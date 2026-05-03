use std::rc::Rc;

use dioxus::prelude::*;
use rust_extensions::date_time::DateTimeAsMicroseconds;
use super::icons::*;
use crate::{
    states::{MainState, DialogState, DialogType},
    views::{render_cpu_graph, render_mem_graph}, utils::format_mem,
};

#[component]
pub fn containers_list(env: Rc<String>) -> Element {
    let mut main_state = consume_context::<Signal<MainState>>();


    let mut dialog_sate = consume_context::<Signal<DialogState>>();


    let mut show_disabled_state = use_signal( || false);


    let show_disabled_state_value = *show_disabled_state.read();


    let now = dioxus_utils::now_date_time();

    let main_state_read_access = main_state.read();
    match main_state_read_access.get_containers() {
        Some(containers) => {
            let containers = containers
                .iter()
                .filter(|itm| {
                    if !show_disabled_state_value && !itm.container.enabled {
                        return false;
                    }
                    true
                })
                .map(|itm| {
                    let color = if itm.container.enabled { "black" } else { "lightgray" };


                    let created_to_render = if let Some(created) = itm.container.created{


                        let created = DateTimeAsMicroseconds::from(created);
                        let duration = now.duration_since(created);
                        let created = created.to_rfc3339();
               

                        let created = &created[0..19];

                        let color = if duration.get_full_days()>=1{
                            "color:green"
                        }else{
                            "color:red"
                        };


                        rsx!{
                            div { style: "padding-top:0 !important; padding-bottom:0 !important; font-size:10px",
                                "Created: {created}"
                            }
                            div { style: "{color}; font-size:10px", "Created: {duration.to_string()}" }
                        }
                        
                    }else{
                        rsx!{}
                    };

                    let state_to_render = if let Some(state) = itm.container.state.as_ref(){
                        rsx!{
                            div { style: "padding-top:0 !important; padding-bottom:0 !important; font-size:10px",
                                "State: {state}"
                            }
                        }
                        }else{
                            rsx!{}
                        };

                        let status_to_render = if let Some(status) = itm.container.status.as_ref(){
                            rsx!{
                                div { style: "padding-top:0 !important; padding-bottom:0 !important; font-size:10px",
                                    "Status: {status}"
                                }
                            }
                            }else{
                                rsx!{}
                            };


                    let mut ports_to_render = Vec::new();

                     if let Some(ports) = itm.container.ports.as_ref() {
                        

                        for port in ports{


                            let public_port = if let Some(public_port) = port.public_port{
                                let ip = port.ip.clone().unwrap_or("*".to_string());
                                format!("{}:{}", ip, public_port)
    
                            }else{
                                "".to_string()
                            };

                            ports_to_render.push(rsx!{
                                div { style: "padding: 2px 10px;",
                                    span {
                                        class: "badge text-bg-secondary",
                                        style: "border-radius: 5px 0px 0px 5px;",
                                        "{port.port_type} {public_port}"
                                    }

                                    span {
                                        class: "badge text-bg-dark",
                                        style: "border-radius: 0px 5px 5px 0px;",
                                        " << {port.private_port}"
                                    }
                                }
                            });
                        }

                    }

                    let cpu_usage = if let Some(usage) = itm.container.cpu.usage {
                        format!("{:.3}", usage)
                    } else {
                        "N/A".to_string()
                    };

                    let mem_limit = if let Some(usage) = itm.container.mem.limit {
                        format_mem(usage)
                    } else {
                        "N/A".to_string()
                    };

                    let mem_usage = if let Some(usage) = itm.container.mem.usage {
                        format_mem(usage)
                    } else {
                        "N/A".to_string()
                    };

                    let id_cloned = itm.container.id.clone();


                    let url_show_logs = itm.url.clone();

                    let vm_name = if let Some(vm_name) = &itm.vm {
                        rsx! {
                            div {
                                div { "{vm_name}" }
                                server_icon_16 {}
                                span { "{itm.url}" }
                            }
                        }   
                    } else {
                        rsx! {
                            div {}
                        }
                    };


       
                    let (cpu_graph, mem_graph) = if let Some(cpu_snapshot) = &itm.container.cpu_usage_history {


                        let mem_limit = if let Some(usage) = itm.container.mem.limit {
                            usage
                        } else {
                            0
                        };

                        let mem_snapshot = itm.container.mem_usage_history.as_ref().unwrap();
                        (
                            rsx! {
                                render_cpu_graph { values: cpu_snapshot.clone() }
                            },
                            rsx! {
                                div {
                                    render_mem_graph {
                                        values: mem_snapshot.clone(),
                                        mem_limit,
                                    }
                                }
                            },
                        )
                    } else {
                        (rsx! {
                            div {}
                        }, rsx! {
                            div {}
                        })
                    };

                    let items = if let Some(labels) = &itm.container.labels {
                        let items = labels.iter().map(|(key, value)| {
                            rsx! {
                                div { style: "font-size:10px; padding:0", "{key}={value}" }
                            }
                        });

                        rsx! {
                            {items}
                        }
                    } else {
                        rsx! {
                            div {}
                        }
                    };

                    let image_cloned = itm.container.image.clone();
                    let env = env.clone();
                    rsx! {
                        tr { style: "border-top: 1px solid lightgray; color: {color}",
                            td {
                                div { "{itm.container.image}" }
                                div { {vm_name} }
                                div { style: "padding-top:0 !important; padding-bottom:0 !important",
                                    {created_to_render}
                                }
                                {state_to_render}
                                {status_to_render}
                                div {
                                    button {
                                        class: "btn btn-sm btn-primary",
                                        onclick: move |_| {
                                            dialog_sate
                                                .write()
                                                .show_dialog(
                                                    format!("Logs of container {}", image_cloned),
                                                    DialogType::ShowLogs {
                                                        env: env.clone(),
                                                        container_id: id_cloned.clone(),
                                                        url: url_show_logs.clone(),
                                                    },
                                                );
                                        },
                                        "Show logs"
                                    }
                                }
                                {ports_to_render.into_iter()}
                            }
                            td { {items} }

                            td {
                                div { style: "cursor:pointer; padding:0",
                                    {cpu_icon()}
                                    ": {cpu_usage}"
                                }
                                div { style: "padding:0", {cpu_graph} }
                                div { style: "cursor:pointer;padding:0;font-size: 12px; margin-top: 5px;",
                                    {memory_icon()}
                                    ": {mem_usage}/{mem_limit}"
                                }
                                div { style: "padding:0", {mem_graph} }
                            }
                        }
                    }
                });

            let show_disabled = if show_disabled_state_value {
                rsx! {
                    button {
                        style: "width: 110px;",
                        class: "btn btn-sm  btn-danger",
                        onclick: move |_| {
                            show_disabled_state.set(false);
                        },
                        "Hide disabled"
                    }
                }
            } else {
                rsx! {
                    button {
                        style: "width: 110px;",
                        class: "btn btn-sm btn-outline-danger",

                        onclick: move |_| {
                            show_disabled_state.set(true);
                        },
                        "Show disabled"
                    }
                }
            };

            let selected_value = main_state.read().get_filter().to_string();

            rsx! {
                table { class: "table table-striped", style: "text-align: left;",
                    tr {
                        th { colspan: 2,
                            table {
                                tr {
                                    td { "Name" }
                                    td { style: "width:100%",
                                        div { class: "input-group",
                                            span { class: "input-group-text", search_icon {} }
                                            input {
                                                class: "form-control form-control-sm",
                                                value: "{selected_value}",
                                                oninput: move |cx| { main_state.write().set_filter(cx.value().to_string()) },
                                            }
                                        }
                                    }
                                    td { {show_disabled} }
                                }
                            }
                        }
                        th { "Cpu/Mem" }
                    }

                    {containers.into_iter()}
                }
            }
        }
        None => {

            rsx! {
                div {}
            }
        }
    }
}

