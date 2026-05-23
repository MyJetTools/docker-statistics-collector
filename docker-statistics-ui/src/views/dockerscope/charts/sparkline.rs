use dioxus::prelude::*;

/// Build line + closed-area SVG path strings for the given values, sized to a (w, h) box with padding.
///
/// Mirrors `buildPath` in `design/charts.jsx`.
pub fn build_path(values: &[f64], w: f64, h: f64, pad: f64) -> (String, String) {
    if values.is_empty() {
        return (String::new(), String::new());
    }
    let max = values.iter().cloned().fold(1.0_f64, f64::max);
    let n = values.len() as f64;
    let denom = (n - 1.0).max(1.0);
    let max_span = (max - 0.0).max(1.0);
    let sx = |i: f64| (i / denom) * (w - pad * 2.0) + pad;
    let sy = |v: f64| h - pad - (v / max_span) * (h - pad * 2.0);

    let mut line = String::with_capacity(values.len() * 14);
    for (i, v) in values.iter().enumerate() {
        let x = sx(i as f64);
        let y = sy(*v);
        if i == 0 {
            line.push_str(&format!("M{:.2},{:.2}", x, y));
        } else {
            line.push_str(&format!(" L{:.2},{:.2}", x, y));
        }
    }
    let area = format!(
        "{line} L{:.2},{:.2} L{:.2},{:.2} Z",
        sx(n - 1.0),
        h,
        sx(0.0),
        h
    );
    (line, area)
}

#[component]
pub fn Sparkline(values: Vec<f64>, color: String, height: u32) -> Element {
    let w = 220.0_f64;
    let h = height as f64;
    let (line, area) = build_path(&values, w, h, 2.0);
    // Stable per-call gradient id — values length+color hash is good enough to avoid collisions on a card.
    let grad_id = format!(
        "sg-{:x}",
        values.len() as u64 ^ color.bytes().map(u64::from).sum::<u64>()
    );
    let view_box = format!("0 0 {} {}", w as i32, height);
    let url = format!("url(#{})", grad_id);

    rsx! {
        svg {
            view_box: "{view_box}",
            preserve_aspect_ratio: "none",
            style: "display:block; width:100%; height:{height}px;",
            defs {
                linearGradient {
                    id: "{grad_id}", x1: "0", y1: "0", x2: "0", y2: "1",
                    stop { offset: "0%", stop_color: "{color}", stop_opacity: "0.35" }
                    stop { offset: "100%", stop_color: "{color}", stop_opacity: "0" }
                }
            }
            path { d: "{area}", fill: "{url}" }
            path {
                d: "{line}", fill: "none",
                stroke: "{color}", stroke_width: "1.2",
                stroke_linejoin: "round", stroke_linecap: "round",
            }
        }
    }
}
