use dioxus::prelude::*;

use crate::METRICS_HISTORY_SIZE;

const HEIGHT: usize = 70;

/// Open-file-descriptor history graph for a container's main process.
///
/// The graph is scaled to the `nofile` limit, so a bar's height reads directly
/// as "how close to the limit" — a steadily climbing line is a descriptor leak
/// marching toward the wall. Bars are coloured green / orange / red by that
/// same ratio. When the limit is unknown the graph falls back to auto-scaling
/// to the highest value in the window.
#[component]
pub fn render_files_graph(fd_limit: i64, values: Vec<i64>) -> Element {
    let scale = if fd_limit > 0 {
        fd_limit as f64
    } else {
        get_max_scale(&values)
    };

    let max_scale_text = (scale as i64).to_string();

    let mut x = METRICS_HISTORY_SIZE - values.len();

    let height_f64 = HEIGHT as f64;

    let mut items = Vec::new();
    for v in values {
        let ratio = v as f64 / scale;

        let y = height_f64 - ratio * height_f64;

        let the_color = if ratio >= 0.9 {
            "rgb(220,0,0)"
        } else if ratio >= 0.7 {
            "rgb(255,140,0)"
        } else {
            "rgb(40,167,69)"
        };

        items.push(rsx! {
            line {
                x1: "{x}",
                x2: "{x}",
                y1: "{y}",
                y2: "{HEIGHT}",
                style: "stroke:{the_color};stroke-width:1"
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
