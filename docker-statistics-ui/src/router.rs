use dioxus::prelude::*;

use crate::selected_vm::SelectedVm;
use crate::states::MainState;

/// URL routing — paths are copy/share-friendly.
///
///   /                                          → no selection (landing)
///   /all                                       → aggregate view (every VM merged)
///   /all/nginx-edge-01-01                      → container under aggregate
///   /vm/vm-prod-app-01                         → single VM
///   /vm/vm-prod-app-01/nginx-edge-01-01        → container under single VM
///
/// Route leaves render nothing; they only sync URL params into `MainState`. The
/// 3-column UI is mounted by `AppShell` (the layout).
#[derive(Routable, Clone, PartialEq, Debug)]
pub enum AppRoute {
    #[layout(crate::AppShell)]
        #[route("/")]
        Home {},
        #[route("/all")]
        AllRoute {},
        #[route("/all/:container_name")]
        AllContainerRoute { container_name: String },
        #[route("/vm/:vm_name")]
        VmRoute { vm_name: String },
        #[route("/vm/:vm_name/:container_name")]
        ContainerRoute {
            vm_name: String,
            container_name: String,
        },
}

#[component]
pub fn Home() -> Element {
    let state = consume_context::<Signal<MainState>>();
    use_effect(move || {
        let mut w = state.to_owned();
        if w.read().get_active_container_name().is_some() {
            w.write().set_active_container_name(None);
        }
    });
    rsx! {}
}

#[component]
pub fn AllRoute() -> Element {
    let state = consume_context::<Signal<MainState>>();
    use_effect(move || {
        sync_all(state);
        let mut w = state.to_owned();
        if w.read().get_active_container_name().is_some() {
            w.write().set_active_container_name(None);
        }
    });
    rsx! {}
}

#[component]
pub fn AllContainerRoute(container_name: String) -> Element {
    let state = consume_context::<Signal<MainState>>();
    let cn = container_name.clone();
    use_effect(use_reactive!(|cn| {
        sync_all(state);
        let mut w = state.to_owned();
        if w.read().get_active_container_name() != Some(cn.as_str()) {
            w.write().set_active_container_name(Some(cn.clone()));
        }
    }));
    rsx! {}
}

#[component]
pub fn VmRoute(vm_name: String) -> Element {
    let state = consume_context::<Signal<MainState>>();
    let vm = vm_name.clone();
    use_effect(use_reactive!(|vm| {
        sync_single_vm(state, &vm);
        let mut w = state.to_owned();
        if w.read().get_active_container_name().is_some() {
            w.write().set_active_container_name(None);
        }
    }));
    rsx! {}
}

#[component]
pub fn ContainerRoute(vm_name: String, container_name: String) -> Element {
    let state = consume_context::<Signal<MainState>>();
    let vm = vm_name.clone();
    let cn = container_name.clone();
    use_effect(use_reactive!(|vm, cn| {
        sync_single_vm(state, &vm);
        let mut w = state.to_owned();
        if w.read().get_active_container_name() != Some(cn.as_str()) {
            w.write().set_active_container_name(Some(cn.clone()));
        }
    }));
    rsx! {}
}

fn sync_single_vm(state: Signal<MainState>, vm: &str) {
    let mut w = state.to_owned();
    let already = {
        let r = w.read();
        !r.is_all_vms_selected()
            && r.get_selected_vm_name()
                .map(|n| n == vm)
                .unwrap_or(false)
    };
    if !already {
        w.write()
            .set_selected_vm(SelectedVm::SingleVm(vm.to_string()));
    }
}

fn sync_all(state: Signal<MainState>) {
    let mut w = state.to_owned();
    let already = w.read().is_all_vms_selected();
    if !already {
        w.write().set_selected_vm(SelectedVm::All);
    }
}
