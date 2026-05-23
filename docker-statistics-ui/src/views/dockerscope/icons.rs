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
