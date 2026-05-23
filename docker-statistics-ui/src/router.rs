use dioxus::prelude::*;

use crate::selected_vm::SelectedVm;
use crate::states::MainState;

/// URL routing — VM and container name appear directly in the path so links
/// can be copied/shared. The path mirrors the user's mental model:
///   /                                     → no selection
///   /vm-prod-app-01                       → VM selected
///   /vm-prod-app-01/nginx-edge-01-01      → VM + container
///
/// Route leaves render nothing; they only sync URL params into MainState. The
/// 3-column UI is mounted by AppShell (the layout).
#[derive(Routable, Clone, PartialEq, Debug)]
pub enum AppRoute {
    #[layout(crate::AppShell)]
        #[route("/")]
        Home {},
        #[route("/:vm_name")]
        VmRoute { vm_name: String },
        #[route("/:vm_name/:container_name")]
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
pub fn VmRoute(vm_name: String) -> Element {
    let state = consume_context::<Signal<MainState>>();
    let vm = vm_name.clone();
    use_effect(use_reactive!(|vm| {
        sync_vm(state, &vm);
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
        sync_vm(state, &vm);
        let mut w = state.to_owned();
        let need_update = w.read().get_active_container_name() != Some(cn.as_str());
        if need_update {
            w.write().set_active_container_name(Some(cn.clone()));
        }
    }));
    rsx! {}
}

fn sync_vm(state: Signal<MainState>, vm: &str) {
    let mut w = state.to_owned();
    let already = w
        .read()
        .get_selected_vm_name()
        .map(|n| n == vm)
        .unwrap_or(false);
    if !already {
        w.write()
            .set_selected_vm(SelectedVm::SingleVm(vm.to_string()));
    }
}
