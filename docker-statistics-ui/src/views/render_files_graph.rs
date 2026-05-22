use dioxus::prelude::*;

use crate::METRICS_HISTORY_SIZE;

const HEIGHT: usize = 70;

/// Open-file-descriptor history graph for a container's main process.
///
/// The graph auto-scales to the highest value in the window (not to the
/// `nofile` limit) — the limit is usually so large that scaling to it would
/// flatten the line and hide a slow leak. The point of this graph is the
/// *shape*: a steadily climbing line means the process is leaking descriptors.
#[component]
pub fn render_files_graph(values: Vec<i64>) -> Element {
    let scale = get_max_scale(&values);

    let max_scale_text = (scale as i64).to_string();

    let mut x = METRICS_HISTORY_SIZE - values.len();

    let height_f64 = HEIGHT as f64;

    let mut items = Vec::new();
    for v in values {
        let v = v as f64;
        let y = v / scale;

        let y = height_f64 - y * height_f64;

        items.push(rsx! {
            line {
                x1: "{x}",
                x2: "{x}",
                y1: "{y}",
                y2: "{HEIGHT}",
                style: "stroke:rgb(255,140,0);stroke-width:1"
            }
        });
        x += 1;
    }

    rsx! {
        svg {
            width: "{METRICS_HISTORY_SIZE}",
            height: "{HEIGHT}",
            view_box: "0 0 {METRICS_HISTORY_SIZE} {HEIGHT}",
            rect {
                width: "{METRICS_HISTORY_SIZE}",
                height: "{HEIGHT}",
                style: "fill:none; stroke-width:1;stroke:rgb(0,0,0)"
            }

            {items.into_iter()},

            text {
                x: "1",
                y: "11",
                fill: "white",
                style: "font-size:10px",
                {max_scale_text.clone()}
            }
            text {
                x: "0",
                y: "10",
                fill: "black",
                style: "font-size:10px",
                {max_scale_text}
            }
        }
    }
}

fn get_max_scale(values: &[i64]) -> f64 {
    let max = *values.iter().max().unwrap_or(&1);

    let max = max as f64;

    if max < 1.0 {
        return 1.0;
    }

    max + max * 0.1
}
