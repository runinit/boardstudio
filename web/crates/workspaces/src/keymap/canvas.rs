use super::view::KeymapView;
use boardstudio_model::model::{Contour, Vec2};
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
        g {
            g { "aria-hidden": "true", "pointer-events": "none",
                for (index, contour) in contours.iter().enumerate() {
                    polygon {
                        key: "outline-{index}",
                        points: contour_points(&contour.points),
                        class: if contour.hole { "fill-canvas stroke-destructive stroke-[0.3] [stroke-dasharray:0.8_0.55]" } else { "fill-geometry-fill stroke-geometry-board stroke-[0.3]" },
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
                    let visible_binding = if binding.as_ref() == "Unassigned" {
                        "—".to_owned()
                    } else {
                        binding.to_string()
                    };
                    let accessible_binding = if binding.as_ref() == "Unassigned" {
                        "Unassigned".to_owned()
                    } else {
                        binding.to_string()
                    };
                    let font_size = (4.4_f64)
                        .min(14.0 / (visible_binding.chars().count().max(1) as f64) * 1.3);
                    rsx! {
                        g {
                            key: "{id}",
                            class: if selected { "cursor-pointer outline-none [&>rect]:fill-selected [&>rect]:stroke-primary [&>rect]:stroke-[1] [&_text]:pointer-events-none" } else { "cursor-pointer outline-none hover:[&>rect]:stroke-primary focus-visible:[&>rect]:fill-selected focus-visible:[&>rect]:stroke-primary focus-visible:[&>rect]:stroke-[1] [&_text]:pointer-events-none" },
                            transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation})",
                            role: "button",
                            tabindex: "0",
                            "aria-label": "Edit key {reference}, binding {accessible_binding}, key ID {id}",
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
                                class: "fill-keycap-face stroke-keycap-rule stroke-[0.55] [vector-effect:non-scaling-stroke]",
                                x: "{-size.x / 2.0}",
                                y: "{-size.y / 2.0}",
                                width: "{size.x}",
                                height: "{size.y}",
                                rx: "1.3",
                                "data-document-color": "{color}",
                            }
                            g { transform: "scale(1 -1)",
                                text {
                                    class: if binding.as_ref() == "Unassigned" {
                                        "fill-muted-foreground font-sans text-[4.6px] font-medium pointer-events-none"
                                    } else {
                                        "fill-keycap-ink font-sans text-[4.6px] font-semibold pointer-events-none"
                                    },
                                    text_anchor: "middle",
                                    dominant_baseline: "central",
                                    font_size: "{font_size}",
                                    "{visible_binding}"
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
