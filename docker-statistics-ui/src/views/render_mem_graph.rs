use dioxus::prelude::*;

use crate::{utils::format_mem, METRICS_HISTORY_SIZE};

const HEIGHT: usize = 70;

#[component]
pub fn render_mem_graph(mem_limit: i64, values: Vec<i64>) -> Element {
    let mem_limit = if mem_limit == 0 {
        get_max_scale(&values)
    } else {
        mem_limit as f64
    };

    let max_scale_text = format_mem(mem_limit as i64);

    let mut x = METRICS_HISTORY_SIZE - values.len();

    let height_f64 = HEIGHT as f64;

    let mut items = Vec::new();
    for v in values {
        let v = v as f64;
        let y = v / mem_limit;

        let y = height_f64 - y * (height_f64 as f64);

        let the_color = if v > mem_limit {
            "rgb(0,0,255)"
        } else {
            "rgb(255,0,0)"
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
