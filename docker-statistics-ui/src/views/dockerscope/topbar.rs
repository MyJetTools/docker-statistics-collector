use dioxus::prelude::*;

use crate::states::{MainState, Prefs, Theme};
use crate::views::dockerscope::icons::*;

#[component]
pub fn Topbar() -> Element {
    let main_state = consume_context::<Signal<MainState>>();
    let cs_ra = main_state.read();

    let envs = cs_ra.envs.items.try_unwrap_as_loaded();
    let selected_env = cs_ra
        .envs
        .get_selected_env()
        .map(|e| e.as_str().to_string())
        .unwrap_or_default();

    let env_options = match envs {
        Some(list) => list
            .iter()
            .map(|e| {
                let e_str = e.as_str().to_string();
                let is_sel = e_str == selected_env;
                rsx! { option { selected: is_sel, value: "{e_str}", "{e_str}" } }
            })
            .collect::<Vec<_>>(),
        None => Vec::new(),
    };

    let selected_vm_label = cs_ra.get_selected_vm_name();
    let active_container_name = cs_ra
        .find_active_container()
        .and_then(|c| c.container.names.first().cloned())
        .map(|n| n.trim_start_matches('/').to_string());

    let totals = compute_fleet_totals(&cs_ra);

    rsx! {
        header { class: "topbar",
            div { class: "brand",
                div { class: "logo", "⌬" }
                span { "dockerscope" }
                span { class: "pulse" }
            }
            select {
                class: "env-select",
                oninput: move |evt| {
                    let v = evt.value();
                    consume_context::<Signal<MainState>>().write().envs.set_active_env(&v);
                },
                {env_options.into_iter()}
            }
            div { class: "crumbs",
                span { "fleet" }
                if let Some(vm) = selected_vm_label.as_ref() {
                    span { class: "sep", "/" }
                    b { "{vm}" }
                }
                if let Some(name) = active_container_name.as_ref() {
                    span { class: "sep", "/" }
                    b { style: "color: var(--accent);", "{name}" }
                }
            }
            div { class: "right",
                div { class: "stats",
                    span { class: "kv",
                        span { class: "swatch", style: "background: var(--accent);" }
                        "vms" b { "{totals.vms}" }
                    }
                    span { class: "kv",
                        span { class: "swatch", style: "background: var(--mem);" }
                        "containers" b { "{totals.containers}" }
                    }
                    span { class: "kv",
                        span { class: "swatch", style: "background: var(--accent);" }
                        "running" b { "{totals.running}" }
                    }
                    span { class: "kv",
                        span { class: "swatch", style: "background: var(--danger);" }
                        "issues" b { "{totals.issues}" }
                    }
                }
                ThemeToggle {}
                button { class: "icon-btn", title: "refresh", {icon_refresh()} }
                button { class: "icon-btn", title: "notifications", {icon_bell()} }
            }
        }
    }
}

#[component]
fn ThemeToggle() -> Element {
    let prefs = consume_context::<Signal<Prefs>>();
    let theme = prefs.read().theme;
    let (icon, title) = match theme {
        Theme::Dark => (icon_sun(), "switch to light theme"),
        Theme::Light => (icon_moon(), "switch to dark theme"),
    };
    rsx! {
        button {
            class: "icon-btn",
            title: "{title}",
            onclick: move |_| {
                let mut prefs = consume_context::<Signal<Prefs>>();
                let mut w = prefs.write();
                w.theme = w.theme.toggle();
                w.save();
            },
            {icon}
        }
    }
}

struct FleetTotals {
    vms: usize,
    containers: usize,
    running: usize,
    issues: usize,
}

fn compute_fleet_totals(cs_ra: &MainState) -> FleetTotals {
    let vms = cs_ra.vms_state.as_ref().map(|m| m.len()).unwrap_or(0);
    let mut containers = 0usize;
    let mut running = 0usize;
    let mut issues = 0usize;
    if let Some(all) = cs_ra.get_all_containers() {
        containers = all.len();
        for c in all {
            let st = c.container.state.as_deref().unwrap_or("").to_ascii_lowercase();
            if st == "running" {
                running += 1;
            }
            if st == "restarting" || st.contains("unhealthy") {
                issues += 1;
            }
        }
    }
    FleetTotals { vms, containers, running, issues }
}
