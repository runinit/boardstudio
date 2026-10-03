//! Read-only daughterboard artwork in resolved host coordinates.
//! Source pads remain module-owned; presentation never creates host circuitry.
use super::LayerVisibility;
use boardstudio_core::model::{PadShape, Vec2};
use boardstudio_core::modules::ResolvedModuleFootprint;
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

pub(super) fn default_hidden_layers() -> BTreeSet<String> {
    [
        "module-outlines",
        "module-clearances",
        "module-holes",
        "module-standoffs",
        "module-findings",
        "module-fab-front",
        "module-fab-back",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct ModuleSourceFootprintsProps {
    module_id: String,
    footprints: Rc<[ResolvedModuleFootprint]>,
}

#[component]
pub(super) fn ModuleSourceFootprints(props: ModuleSourceFootprintsProps) -> Element {
    let visibility = use_context::<LayerVisibility>();
    let hidden = (visibility.modules_hidden)();
    let host_hidden = (visibility.hidden)();
    rsx! {
        g { class: "m1-module-pcb-overlay", "data-module-id": "{props.module_id}",
            if !hidden.contains("module-footprints") {
                for footprint in props.footprints.iter() {
                    {
                        let back = footprint.side == boardstudio_core::model::Side::Back;
                        let face = if back { "back" } else { "front" };
                        let side_transform = if back { "scale(-1 1)" } else { "" };
                        let pose = footprint.pose;
                        let copper = if back { "B.Cu" } else { "F.Cu" };
                        let opposite = if back { "F.Cu" } else { "B.Cu" };
                        rsx! {
                            g {
                                key: "{footprint.id}",
                                class: "m1-module-source-footprint is-{face}",
                                transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation}) {side_transform}",
                                "data-source-part": "{footprint.source_part_id}",
                                polygon {
                                    class: if host_hidden.contains("Courtyards") { "m1-module-footprint-courtyard is-host-hidden" } else { "m1-module-footprint-courtyard" },
                                    points: points(&footprint.courtyard),
                                }
                                for (index, surface) in footprint.surfaces.iter().enumerate() {
                                    {
                                        let front = surface.layer.starts_with("F.");
                                        let silk = surface.layer.ends_with(".SilkS");
                                        let visibility_id = match (silk, front) {
                                            (true, true) => "module-silkscreen-front",
                                            (true, false) => "module-silkscreen-back",
                                            (false, true) => "module-fab-front",
                                            (false, false) => "module-fab-back",
                                        };
                                        let is_reference = !surface.text.is_empty() && surface.text == footprint.reference;
                                        let visible = !hidden.contains(visibility_id)
                                            && !host_hidden.contains(&surface.layer)
                                            && !(is_reference && host_hidden.contains("References"));
                                        let kind = if silk { "is-silkscreen" } else { "is-fabrication" };
                                        let origin = surface.points.first().copied().unwrap_or_default();
                                        let size = if surface.text_size == 0.0 { 1.0 } else { surface.text_size };
                                        let width = if surface.width == 0.0 { 0.15 } else { surface.width };
                                        rsx! {
                                            if visible {
                                                if !surface.text.is_empty() {
                                                    text {
                                                        key: "surface-{index}", class: "m1-module-footprint-art is-text {kind}",
                                                        "data-source-layer": "{surface.layer}",
                                                        x: "{origin.x}", y: "{origin.y}", font_size: "{size}",
                                                        transform: "rotate({surface.rotation} {origin.x} {origin.y}) scale(1,-1)",
                                                        "{surface.text}"
                                                    }
                                                } else if surface.filled {
                                                    polygon {
                                                        key: "surface-{index}", class: "m1-module-footprint-art is-filled {kind}",
                                                        "data-source-layer": "{surface.layer}", points: points(&surface.points),
                                                    }
                                                } else {
                                                    path {
                                                        key: "surface-{index}", class: "m1-module-footprint-art {kind}",
                                                        "data-source-layer": "{surface.layer}", d: path(&surface.points), stroke_width: "{width}",
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                for pad in &footprint.pads {
                                    {
                                        let copper_visible = !host_hidden.contains("Pads")
                                            && (!host_hidden.contains(copper) || (pad.drill.is_some() && !host_hidden.contains(opposite)));
                                        let radius = match pad.shape {
                                            PadShape::Circle | PadShape::Oval => pad.size.x.min(pad.size.y) / 2.0,
                                            PadShape::Roundrect => pad.size.x.min(pad.size.y) / 4.0,
                                            PadShape::Rect => 0.0,
                                        };
                                        let rotation = pad.rotation.unwrap_or(0.0);
                                        rsx! {
                                            g { key: "{pad.id}", class: "m1-module-source-pad", "data-pad-number": "{pad.number}",
                                                transform: "translate({pad.at.x} {pad.at.y}) rotate({rotation})",
                                                if copper_visible {
                                                    rect { x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}", rx: "{radius}" }
                                                }
                                                if let Some(drill) = pad.drill.filter(|_| !host_hidden.contains("Holes")) {
                                                    circle { class: "m1-module-source-drill", r: "{drill / 2.0}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                if !host_hidden.contains("References") && !footprint.surfaces.iter().any(|surface| surface.text == footprint.reference) {
                                    text { class: "m1-module-source-reference", x: "0", y: "-2", "{footprint.reference}" }
                                }
                                title { "{footprint.reference} · {footprint.name} · source module footprint" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ")
}

fn path(points: &[Vec2]) -> String {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let command = if index == 0 { 'M' } else { 'L' };
            format!("{command}{} {}", point.x, point.y)
        })
        .collect::<Vec<_>>()
        .join(" ")
}
