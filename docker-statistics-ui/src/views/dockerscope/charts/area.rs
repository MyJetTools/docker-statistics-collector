use dioxus::prelude::*;

use super::build_path;

/// Area chart for the right-side detail panel. Fixed virtual width — the SVG `preserveAspectRatio="none"`
/// stretches to the parent container width, which is good enough without a ResizeObserver hookup.
#[component]
pub fn AreaChart(values: Vec<f64>, color: String, height: u32, unit: String) -> Element {
    let _ = unit; // reserved for hover tooltip in a later pass
    let w = 600.0_f64;
    let h = height as f64;
    let pad = 4.0;
    let (line, area) = build_path(&values, w, h, pad);

    let grad_id = format!(
        "ag-{:x}",
        values.len() as u64 ^ color.bytes().map(u64::from).sum::<u64>()
    );
    let url = format!("url(#{})", grad_id);

    let max = values.iter().cloned().fold(1.0_f64, f64::max);
    let last_v = values.last().copied().unwrap_or(0.0);
    let last_x = w - pad;
    let last_y = h - pad - (last_v / max.max(1.0)) * (h - pad * 2.0);

    let view_box = format!("0 0 {} {}", w as i32, height);

    rsx! {
        svg {
            view_box: "{view_box}",
            preserve_aspect_ratio: "none",
            style: "display:block; width:100%; height:{height}px;",
            defs {
                linearGradient {
                    id: "{grad_id}", x1: "0", y1: "0", x2: "0", y2: "1",
                    stop { offset: "0%", stop_color: "{color}", stop_opacity: "0.32" }
                    stop { offset: "100%", stop_color: "{color}", stop_opacity: "0" }
                }
            }
            g {
                line {
                    x1: "0", x2: "{w}", y1: "{h * 0.25}", y2: "{h * 0.25}",
                    stroke: "#1d232c", stroke_dasharray: "2 4", stroke_width: "1",
                }
                line {
                    x1: "0", x2: "{w}", y1: "{h * 0.5}", y2: "{h * 0.5}",
                    stroke: "#1d232c", stroke_dasharray: "2 4", stroke_width: "1",
                }
                line {
                    x1: "0", x2: "{w}", y1: "{h * 0.75}", y2: "{h * 0.75}",
                    stroke: "#1d232c", stroke_dasharray: "2 4", stroke_width: "1",
                }
            }
            path { d: "{area}", fill: "{url}" }
            path {
                d: "{line}", fill: "none",
                stroke: "{color}", stroke_width: "1.4",
                stroke_linejoin: "round", stroke_linecap: "round",
            }
            circle {
                cx: "{last_x}", cy: "{last_y}", r: "3",
                fill: "{color}", stroke: "#0a0b0d", stroke_width: "2",
            }
        }
    }
}
