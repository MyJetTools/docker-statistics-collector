use dioxus::prelude::*;

pub fn search_icon() -> Element {
    rsx! {
        img { src: "/assets/img/ico-search.svg" }
    }
}

pub fn server_icon() -> Element {
    rsx! {
        img { src: "/assets/img/ico-server.svg" }
    }
}

pub fn server_icon_16() -> Element {
    rsx! {
        img { src: "/assets/img/ico-server-16.svg" }
    }
}

pub fn memory_icon() -> Element {
    rsx! {
        img { src: "/assets/img/ico-memory.svg" }
    }
}

pub fn cpu_icon() -> Element {
    rsx! {
        img { src: "/assets/img/ico-cpu.svg" }
    }
}
