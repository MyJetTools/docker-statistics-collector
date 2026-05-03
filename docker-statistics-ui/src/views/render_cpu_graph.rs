use dioxus::prelude::*;

use crate::METRICS_HISTORY_SIZE;

const HEIGHT: usize = 70;

#[component]
pub fn render_cpu_graph(values: Vec<f64>) -> Element {
    let max_scale = get_max_scale(&values);

    let max_scale_text = format!("{:.6}", max_scale);

    let max_scale = max_scale as f64;

    let mut x = METRICS_HISTORY_SIZE - values.len();

    let mut items = Vec::new();
    let h_f64 = HEIGHT as f64;
    for v in values {
        let v = v as f64;

        let y = h_f64 - v / max_scale * h_f64;
        items.push(rsx! {
            line {
                x1: "{x}",
                x2: "{x}",
                y1: "{y}",
                y2: "{HEIGHT}",
                style: "stroke:rgb(0,128,0);stroke-width:1"
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

fn get_max_scale(values: &[f64]) -> f64 {
    if values.len() == 0 {
        return 1.0;
    }

    let mut max = *values.first().unwrap();

    for v in values {
        if *v > max {
            max = *v;
        }
    }

    if max < 0.001 {
        return 0.001;
    }

    max + max * 0.1
}
