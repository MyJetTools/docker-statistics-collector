use dioxus::prelude::*;

const STROKE: &str =
    "stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.6; fill: none; stroke: currentColor;";

pub fn icon_server() -> Element {
    rsx! {
        svg {
            width: "14", height: "14", view_box: "0 0 24 24",
            style: STROKE,
            rect { x: "3", y: "4",  width: "18", height: "6", rx: "1.5" }
            rect { x: "3", y: "14", width: "18", height: "6", rx: "1.5" }
            circle { cx: "7", cy: "7",  r: "0.7", fill: "currentColor", stroke: "none" }
            circle { cx: "7", cy: "17", r: "0.7", fill: "currentColor", stroke: "none" }
        }
    }
}

/// Host-disk / volume glyph (storage drive with a gauge), inline so it inherits
/// the row's `currentColor`. Adapted from public/assets/img/volume.svg.
pub fn icon_disk() -> Element {
    rsx! {
        svg {
            width: "11", height: "11", view_box: "0 0 24 24",
            style: "stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.7; fill: none; stroke: currentColor;",
            rect { x: "3.5", y: "1.5", width: "17", height: "21", rx: "2" }
            circle { cx: "12", cy: "10", r: "4.6" }
            line { x1: "13.4", y1: "11.4", x2: "16.3", y2: "14.3" }
            circle { cx: "12", cy: "10", r: "0.9", fill: "currentColor", stroke: "none" }
            line { x1: "7.2", y1: "18.7", x2: "16.8", y2: "18.7" }
        }
    }
}

/// CPU-chip glyph, inline so it inherits `currentColor`. Adapted from
/// public/assets/img/ico-cpu.svg (a fill icon).
pub fn icon_cpu() -> Element {
    rsx! {
        svg {
            width: "12", height: "12", view_box: "0 0 16 16",
            style: "fill: currentColor;",
            path { d: "M5 0a.5.5 0 0 1 .5.5V2h1V.5a.5.5 0 0 1 1 0V2h1V.5a.5.5 0 0 1 1 0V2h1V.5a.5.5 0 0 1 1 0V2A2.5 2.5 0 0 1 14 4.5h1.5a.5.5 0 0 1 0 1H14v1h1.5a.5.5 0 0 1 0 1H14v1h1.5a.5.5 0 0 1 0 1H14v1h1.5a.5.5 0 0 1 0 1H14a2.5 2.5 0 0 1-2.5 2.5v1.5a.5.5 0 0 1-1 0V14h-1v1.5a.5.5 0 0 1-1 0V14h-1v1.5a.5.5 0 0 1-1 0V14h-1v1.5a.5.5 0 0 1-1 0V14A2.5 2.5 0 0 1 2 11.5H.5a.5.5 0 0 1 0-1H2v-1H.5a.5.5 0 0 1 0-1H2v-1H.5a.5.5 0 0 1 0-1H2v-1H.5a.5.5 0 0 1 0-1H2A2.5 2.5 0 0 1 4.5 2V.5A.5.5 0 0 1 5 0zm-.5 3A1.5 1.5 0 0 0 3 4.5v7A1.5 1.5 0 0 0 4.5 13h7a1.5 1.5 0 0 0 1.5-1.5v-7A1.5 1.5 0 0 0 11.5 3h-7zM5 6.5A1.5 1.5 0 0 1 6.5 5h3A1.5 1.5 0 0 1 11 6.5v3A1.5 1.5 0 0 1 9.5 11h-3A1.5 1.5 0 0 1 5 9.5v-3zM6.5 6a.5.5 0 0 0-.5.5v3a.5.5 0 0 0 .5.5h3a.5.5 0 0 0 .5-.5v-3a.5.5 0 0 0-.5-.5h-3z" }
        }
    }
}

/// Memory-module (RAM stick) glyph, inline so it inherits `currentColor`.
/// Adapted from public/assets/img/ico-memory.svg (a fill icon).
pub fn icon_memory() -> Element {
    rsx! {
        svg {
            width: "12", height: "12", view_box: "0 0 16 16",
            style: "fill: currentColor;",
            path { d: "M1 3a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h4.586a1 1 0 0 0 .707-.293l.353-.353a.5.5 0 0 1 .708 0l.353.353a1 1 0 0 0 .707.293H15a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H1Zm.5 1h3a.5.5 0 0 1 .5.5v4a.5.5 0 0 1-.5.5h-3a.5.5 0 0 1-.5-.5v-4a.5.5 0 0 1 .5-.5Zm5 0h3a.5.5 0 0 1 .5.5v4a.5.5 0 0 1-.5.5h-3a.5.5 0 0 1-.5-.5v-4a.5.5 0 0 1 .5-.5Zm4.5.5a.5.5 0 0 1 .5-.5h3a.5.5 0 0 1 .5.5v4a.5.5 0 0 1-.5.5h-3a.5.5 0 0 1-.5-.5v-4ZM2 10v2H1v-2h1Zm2 0v2H3v-2h1Zm2 0v2H5v-2h1Zm3 0v2H8v-2h1Zm2 0v2h-1v-2h1Zm2 0v2h-1v-2h1Zm2 0v2h-1v-2h1Z" }
        }
    }
}

/// Network glyph (nested squares / node), inline so it inherits the row's
/// `currentColor`. Adapted from public/assets/img/network.svg (a fill icon).
pub fn icon_network() -> Element {
    rsx! {
        svg {
            width: "11", height: "11", view_box: "0 0 16 16",
            style: "fill: currentColor;",
            path { fill_rule: "evenodd", d: "M9 7V5H7v2H5v4h6V7H9zm-9 9h16V0H0v16zm2-2V2h12v12H2z" }
        }
    }
}

pub fn icon_search() -> Element {
    rsx! {
        svg {
            width: "13", height: "13", view_box: "0 0 24 24",
            style: "stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.8; fill: none; stroke: currentColor;",
            circle { cx: "11", cy: "11", r: "7" }
            path { d: "M21 21l-4.3-4.3" }
        }
    }
}

pub fn icon_bell() -> Element {
    rsx! {
        svg {
            width: "14", height: "14", view_box: "0 0 24 24",
            style: STROKE,
            path { d: "M6 8a6 6 0 1 1 12 0c0 7 3 9 3 9H3s3-2 3-9z" }
            path { d: "M10 21a2 2 0 0 0 4 0" }
        }
    }
}

pub fn icon_refresh() -> Element {
    rsx! {
        svg {
            width: "14", height: "14", view_box: "0 0 24 24",
            style: STROKE,
            path { d: "M21 12a9 9 0 1 1-3-6.7" }
            path { d: "M21 4v5h-5" }
        }
    }
}

pub fn icon_copy() -> Element {
    rsx! {
        svg {
            width: "12", height: "12", view_box: "0 0 24 24",
            style: STROKE,
            rect { x: "8", y: "8", width: "12", height: "12", rx: "1.5" }
            path { d: "M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3" }
        }
    }
}

pub fn icon_ext() -> Element {
    rsx! {
        svg {
            width: "11", height: "11", view_box: "0 0 24 24",
            style: "stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.8; fill: none; stroke: currentColor;",
            path { d: "M14 4h6v6" }
            path { d: "M20 4l-9 9" }
            path { d: "M19 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h5" }
        }
    }
}

pub fn icon_play() -> Element {
    rsx! {
        svg {
            width: "12", height: "12", view_box: "0 0 24 24",
            fill: "currentColor",
            path { d: "M7 4v16l13-8z" }
        }
    }
}

pub fn icon_pause() -> Element {
    rsx! {
        svg {
            width: "12", height: "12", view_box: "0 0 24 24",
            fill: "currentColor",
            rect { x: "6", y: "4", width: "4", height: "16" }
            rect { x: "14", y: "4", width: "4", height: "16" }
        }
    }
}

pub fn icon_terminal() -> Element {
    rsx! {
        svg {
            width: "12", height: "12", view_box: "0 0 24 24",
            style: "stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.8; fill: none; stroke: currentColor;",
            path { d: "M4 7l4 5-4 5" }
            path { d: "M12 17h8" }
        }
    }
}

pub fn icon_logs() -> Element {
    rsx! {
        svg {
            width: "12", height: "12", view_box: "0 0 24 24",
            style: "stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.8; fill: none; stroke: currentColor;",
            path { d: "M4 6h16M4 10h16M4 14h10M4 18h13" }
        }
    }
}

pub fn icon_arrow_down() -> Element {
    rsx! {
        svg {
            width: "10", height: "10", view_box: "0 0 24 24",
            style: "stroke-linecap: round; stroke-linejoin: round; stroke-width: 2; fill: none; stroke: currentColor;",
            path { d: "M12 5v14M5 12l7 7 7-7" }
        }
    }
}

pub fn icon_sun() -> Element {
    rsx! {
        svg {
            width: "14", height: "14", view_box: "0 0 24 24",
            style: STROKE,
            circle { cx: "12", cy: "12", r: "4" }
            path { d: "M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" }
        }
    }
}

pub fn icon_moon() -> Element {
    rsx! {
        svg {
            width: "14", height: "14", view_box: "0 0 24 24",
            style: STROKE,
            path { d: "M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" }
        }
    }
}

pub fn icon_more() -> Element {
    rsx! {
        svg {
            width: "12", height: "12", view_box: "0 0 24 24",
            fill: "currentColor",
            circle { cx: "5", cy: "12", r: "1.5" }
            circle { cx: "12", cy: "12", r: "1.5" }
            circle { cx: "19", cy: "12", r: "1.5" }
        }
    }
}
