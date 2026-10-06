use super::view::KeymapView;
use boardstudio_core::model::{Contour, Vec2};
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

/// Render key targets into the existing board SVG coordinate system.
#[component]
pub fn KeymapCanvas(
    view: Rc<KeymapView>,
    contours: Rc<[Contour]>,
    selected_ids: BTreeSet<String>,
    on_select_key: EventHandler<String>,
) -> Element {
    rsx! {
        g { class: "m1-keymap-layout",
            g { class: "m1-keymap-outline", "aria-hidden": "true", "pointer-events": "none",
                for (index, contour) in contours.iter().enumerate() {
                    polygon {
                        key: "outline-{index}",
                        points: contour_points(&contour.points),
                        class: if contour.hole { "m1-outline is-hole" } else { "m1-outline" },
                    }
                }
            }
            for key in &view.keys {
                {
                    let id = key.id.clone();
                    let click_id = id.clone();
                    let keyboard_id = id.clone();
                    let reference = key.reference.clone();
                    let selected = selected_ids.contains(id.as_ref());
                    let color = key.color.clone();
                    let pose = key.pose;
                    let size = key.size;
                    let binding = key.binding_title.clone();
                    let foreground = foreground_color(&color);
                    let font_size = (4.0_f64).min(13.0 / (binding.chars().count().max(1) as f64) * 1.3);
                    rsx! {
                        g {
                            key: "{id}",
                            class: if selected { "m1-keymap-key is-selected" } else { "m1-keymap-key" },
                            transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation})",
                            role: "button",
                            tabindex: "0",
                            "aria-label": "Edit key {reference}",
                            onclick: move |event| {
                                event.stop_propagation();
                                on_select_key.call(click_id.to_string());
                            },
                            onkeydown: move |event: KeyboardEvent| {
                                let pressed = event.data().key().to_string();
                                if pressed == "Enter" || pressed == " " {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    on_select_key.call(keyboard_id.to_string());
                                }
                            },
                            rect {
                                x: "{-size.x / 2.0}",
                                y: "{-size.y / 2.0}",
                                width: "{size.x}",
                                height: "{size.y}",
                                rx: "1.3",
                                fill: "{color}",
                            }
                            g { transform: "scale(1 -1)",
                                text {
                                    text_anchor: "middle",
                                    dominant_baseline: "central",
                                    fill: "{foreground}",
                                    font_size: "{font_size}",
                                    if binding.is_empty() { "—" } else { "{binding}" }
                                }
                                text {
                                    text_anchor: "middle",
                                    y: "6",
                                    font_size: "2.1",
                                    fill: "{foreground}",
                                    "{reference}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn contour_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ")
}

fn foreground_color(color: &str) -> &'static str {
    let Some(hex) = color.strip_prefix('#') else {
        return "#ffffff";
    };
    let Ok(rgb) = u32::from_str_radix(hex, 16) else {
        return "#ffffff";
    };
    let red = ((rgb >> 16) & 255) as f64;
    let green = ((rgb >> 8) & 255) as f64;
    let blue = (rgb & 255) as f64;
    if red * 0.299 + green * 0.587 + blue * 0.114 > 150.0 {
        "#182331"
    } else {
        "#ffffff"
    }
}
