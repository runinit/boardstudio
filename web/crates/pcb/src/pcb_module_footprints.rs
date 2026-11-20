//! Read-only daughterboard artwork in resolved host coordinates.
//! Source pads remain module-owned; presentation never creates host circuitry.
use super::LayerVisibility;
use boardstudio_application::AcceptedSnapshot;
use boardstudio_model::model::{PadShape, Severity, Vec2};
use dioxus::prelude::*;
use std::collections::BTreeSet;

pub fn default_hidden_layers() -> BTreeSet<String> {
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
pub struct ModuleSourceFootprintsProps {
    module_id: String,
    snapshot: AcceptedSnapshot,
    on_select: EventHandler<String>,
}

#[component]
pub fn ModuleSourceFootprints(props: ModuleSourceFootprintsProps) -> Element {
    let visibility = use_context::<LayerVisibility>();
    let hidden = (visibility.modules_hidden)();
    let host_hidden = (visibility.hidden)();
    let on_select = props.on_select;
    let Some(module) = props
        .snapshot
        .scene
        .module_scenes
        .iter()
        .find(|module| module.id == props.module_id)
    else {
        return rsx! {};
    };
    let module_label = props
        .snapshot
        .document
        .modules
        .iter()
        .find(|instance| instance.id == props.module_id)
        .and_then(|instance| {
            props
                .snapshot
                .document
                .module_definitions
                .iter()
                .find(|definition| definition.id == instance.definition_id)
                .map(|definition| definition.name.clone())
        })
        .unwrap_or_else(|| props.module_id.clone());
    rsx! {
        g {
            class: "focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-2",
            "data-module-id": "{props.module_id}",
            role: "button",
            tabindex: "0",
            "aria-label": "Select mounted module {module_label}",
            style: "cursor: pointer",
            onpointerdown: move |event: PointerEvent| event.stop_propagation(),
            onclick: { let id = props.module_id.clone(); move |_| on_select.call(id.clone()) },
            onkeydown: { let id = props.module_id.clone(); move |event: KeyboardEvent| {
                if event.key() == Key::Enter || event.key() == Key::Character(" ".into()) {
                    event.prevent_default();
                    on_select.call(id.clone());
                }
            } },
            if !hidden.contains("module-outlines") {
                for (index, outline) in module.board.iter().enumerate() {
                    polygon {
                        key: "outline-{index}",
                        class: "fill-none stroke-primary stroke-[0.22] [stroke-dasharray:0.8_0.45] [vector-effect:non-scaling-stroke]",
                        points: points(&outline.points),
                    }
                }
            }
            if !hidden.contains("module-clearances") {
                for volume in module.volumes.iter().chain(&module.openings) {
                    polygon {
                        key: "clearance-{volume.id}", class: "fill-[#db9d49]/[0.045] stroke-warning-dark stroke-[0.18] [stroke-dasharray:0.6_0.55] [vector-effect:non-scaling-stroke]",
                        points: points(&volume.geometry.points), "data-qualified": "{volume.qualified}",
                    }
                }
            }
            if !hidden.contains("module-holes") {
                for mount in &module.mounts {
                    g { key: "hole-{mount.source_id}", class: "[&_circle]:fill-canvas [&_circle]:stroke-primary [&_circle]:stroke-[0.22] [&_circle]:[vector-effect:non-scaling-stroke] [&_path]:fill-none [&_path]:stroke-primary [&_path]:stroke-[0.14] [&_path]:[vector-effect:non-scaling-stroke]",
                        title { "Module mounting hole {mount.source_id} · source drill {mount.diameter} mm" }
                        circle { cx: "{mount.at.x}", cy: "{mount.at.y}", r: "{mount.diameter / 2.0}" }
                        path { d: "M{mount.at.x - mount.diameter / 2.0} {mount.at.y}h{mount.diameter}M{mount.at.x} {mount.at.y - mount.diameter / 2.0}v{mount.diameter}" }
                    }
                }
            }
            if !hidden.contains("module-standoffs") {
                for (index, support) in module.mount_supports.iter().enumerate() {
                    g { key: "support-{support.mount_id}-{index}", class: "[&_circle]:fill-[#69b5a8]/[0.14] [&_circle]:stroke-primary [&_circle]:stroke-[0.22] [&_circle]:[vector-effect:non-scaling-stroke]",
                        title { "Designer-selected support at {support.mount_id} · outer diameter {support.outer_diameter} mm · hole {support.hole_diameter} mm · Z {support.z} mm · height {support.height} mm" }
                        circle { cx: "{support.at.x}", cy: "{support.at.y}", r: "{support.outer_diameter / 2.0}" }
                        circle { class: "!fill-canvas", cx: "{support.at.x}", cy: "{support.at.y}", r: "{support.hole_diameter / 2.0}" }
                    }
                }
            }
            if !hidden.contains("module-footprints") {
                for footprint in &module.footprints {
                    {
                        let back = footprint.side == boardstudio_model::model::Side::Back;
                        let side_transform = if back { "scale(-1 1)" } else { "" };
                        let pose = footprint.pose;
                        let copper = if back { "B.Cu" } else { "F.Cu" };
                        let opposite = if back { "F.Cu" } else { "B.Cu" };
                        rsx! {
                            g {
                                key: "{footprint.id}",
                                class: "opacity-[0.86]",
                                transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation}) {side_transform}",
                                "data-source-part": "{footprint.source_part_id}",
                                polygon {
                                    class: if host_hidden.contains("Courtyards") { "fill-transparent stroke-none stroke-[0.13] [stroke-dasharray:0.3_0.2] [vector-effect:non-scaling-stroke]" } else { "fill-transparent stroke-geometry-outline stroke-[0.13] [stroke-dasharray:0.3_0.2] [vector-effect:non-scaling-stroke]" },
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
                                        let stroke = if silk { "stroke-geometry-label" } else { "stroke-geometry-graphic" };
                                        let origin = surface.points.first().copied().unwrap_or_default();
                                        let size = if surface.text_size == 0.0 { 1.0 } else { surface.text_size };
                                        let width = if surface.width == 0.0 { 0.15 } else { surface.width };
                                        rsx! {
                                            if visible {
                                                if !surface.text.is_empty() {
                                                    text {
                                                        key: "surface-{index}", class: "fill-geometry-label stroke-none [vector-effect:non-scaling-stroke] pointer-events-none",
                                                        "data-source-layer": "{surface.layer}",
                                                        x: "{origin.x}", y: "{origin.y}", font_size: "{size}",
                                                        transform: "rotate({surface.rotation} {origin.x} {origin.y}) scale(1,-1)",
                                                        "{surface.text}"
                                                    }
                                                } else if surface.filled {
                                                    polygon {
                                                        key: "surface-{index}", class: "fill-[#becbd0]/[0.08] {stroke} stroke-[0.15] [vector-effect:non-scaling-stroke] pointer-events-none",
                                                        "data-source-layer": "{surface.layer}", points: points(&surface.points),
                                                    }
                                                } else {
                                                    path {
                                                        key: "surface-{index}", class: "fill-none {stroke} stroke-[0.15] [vector-effect:non-scaling-stroke] pointer-events-none",
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
                                            g { key: "{pad.id}", class: "[&_rect]:fill-geometry-part [&_rect]:[fill-opacity:0.6] [&_rect]:stroke-card [&_rect]:stroke-[0.08] [&_rect]:[vector-effect:non-scaling-stroke]", "data-pad-number": "{pad.number}",
                                                transform: "translate({pad.at.x} {pad.at.y}) rotate({rotation})",
                                                if copper_visible {
                                                    rect { x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}", rx: "{radius}" }
                                                }
                                                if let Some(drill) = pad.drill.filter(|_| !host_hidden.contains("Holes")) {
                                                    circle { class: "fill-canvas stroke-geometry-outline stroke-[0.1] [vector-effect:non-scaling-stroke]", r: "{drill / 2.0}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                if !host_hidden.contains("References") && !footprint.surfaces.iter().any(|surface| surface.text == footprint.reference) {
                                    text { class: "fill-geometry-label stroke-none font-mono text-[1px] [text-anchor:middle] pointer-events-none", x: "0", y: "-2", "{footprint.reference}" }
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

#[component]
pub fn PcbFindingMarkers(snapshot: AcceptedSnapshot, board_id: String) -> Element {
    let visibility = use_context::<LayerVisibility>();
    let module_findings_hidden = (visibility.modules_hidden)().contains("module-findings");
    let module_ids = snapshot
        .document
        .modules
        .iter()
        .filter(|module| module.host_board_id == board_id)
        .map(|module| module.id.as_str())
        .collect::<BTreeSet<_>>();
    // Mirror Workbench.tsx's moduleFindingIds projection: classify every
    // finding against mounted-module instance IDs on this host board, then
    // hide only those error markers when the module layer is off. Host error
    // markers stay visible in PCB independently of module-layer visibility.
    let module_finding_ids = snapshot
        .scene
        .findings
        .iter()
        .filter(|finding| {
            finding
                .target_ids
                .iter()
                .any(|id| module_ids.contains(id.as_str()))
        })
        .map(|finding| finding.id.as_str())
        .collect::<BTreeSet<_>>();
    let visible_error_ids = snapshot
        .scene
        .findings
        .iter()
        .filter(|finding| {
            finding.severity == Severity::Error
                && !(module_findings_hidden && module_finding_ids.contains(finding.id.as_str()))
        })
        .map(|finding| finding.id.as_str())
        .collect::<BTreeSet<_>>();
    rsx! {
        for marker in snapshot.scene.finding_markers.iter().filter(|marker| {
            marker.board_id == board_id
                && visible_error_ids.contains(marker.finding_id.as_str())
        }) {
            g {
                key: "{marker.finding_id}",
                class: "wb-outline-finding",
                "data-finding-id": "{marker.finding_id}",
                for (index, contour) in marker.contours.iter().enumerate() {
                    polygon { key: "contour-{index}", points: points(&contour.points) }
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
