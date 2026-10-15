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
                            class: if selected { "m1-keymap-key is-selected" } else { "m1-keymap-key" },
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
                                class: "m1-keymap-key-face",
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
                                        "m1-keymap-binding-label is-unassigned"
                                    } else {
                                        "m1-keymap-binding-label"
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
