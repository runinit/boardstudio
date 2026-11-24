//! Read-only host scene projection for the currently accepted PCB board.
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_model::model::{PadShape, Side, Vec2};
use boardstudio_web_ui_shared::footprint_graphics::FootprintGraphics;
use boardstudio_web_ui_shared::svg_points::polygon_points;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcbPartHit {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub generation: u64,
    pub part_id: String,
    pub additive: bool,
    pub range: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcbPartPointerDown {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub generation: u64,
    pub part_id: String,
    pub pointer_id: i64,
    pub client_x: i32,
    pub client_y: i32,
    pub additive: bool,
    pub range: bool,
}

#[derive(Props, Clone, PartialEq)]
pub struct PcbSceneProps {
    pub snapshot: AcceptedSnapshot,
    pub scope: Scope,
    pub selected_ids: Vec<String>,
    pub generation: u64,
    pub on_part_hit: EventHandler<PcbPartHit>,
    pub on_part_pointer_down: EventHandler<PcbPartPointerDown>,
    pub on_module_select: EventHandler<String>,
}

#[component]
pub fn PcbScene(props: PcbSceneProps) -> Element {
    let snapshot = props.snapshot;
    let scope = props.scope;
    let selected_ids = props.selected_ids;
    let on_part_hit = props.on_part_hit;
    let on_part_pointer_down = props.on_part_pointer_down;
    let on_module_select = props.on_module_select;
    let generation = props.generation;
    let hidden_layers = (use_context::<super::LayerVisibility>().hidden)();

    if snapshot.session_epoch != scope.session_epoch || snapshot.document.id != scope.document_id {
        return rsx! {
            g { "data-scene-status": "stale", role: "status" }
        };
    }

    let document = boardstudio_application::resolved_document(&snapshot.document);
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
    else {
        return rsx! {
            g { "data-scene-status": "unavailable", role: "status" }
        };
    };
    let board_id = board.id.as_str();
    let contours = snapshot
        .scene
        .board_contours
        .iter()
        .find(|entry| entry.board_id == board_id)
        .map(|entry| entry.contours.as_slice())
        .or_else(|| (document.boards.len() == 1).then_some(snapshot.scene.contours.as_slice()))
        .unwrap_or_default();
    let member_ids: BTreeSet<&str> = board.part_ids.iter().map(String::as_str).collect();
    let scene_transforms = snapshot.scene.transforms.as_slice();
    rsx! {
        g { "data-board-id": board_id, "data-scene-status": "ready",
            if !hidden_layers.contains("Edge.Cuts") {
                for (index, contour) in contours.iter().enumerate() {
                    polygon {
                        key: "outline-{index}",
                        "data-layer": "Edge.Cuts",
                        points: polygon_points(&contour.points),
                        class: if contour.hole { "fill-canvas stroke-destructive stroke-[0.3] [stroke-dasharray:0.8_0.55]" } else { "fill-geometry-fill stroke-geometry-board stroke-[0.3]" },
                    }
                }
            }
            for part in document.parts.iter().filter(|part| member_ids.contains(part.id.as_str())) {
                {
                    let definition = document.definitions.iter().find(|definition| definition.id == part.definition_id);
                    let pose = scene_transforms
                        .iter()
                        .find(|transform| transform.id == part.id)
                        .map(|transform| transform.pose)
                        .unwrap_or(part.pose);
                    let side_transform = if matches!(&part.side, Side::Back) { "scale(-1 1)" } else { "" };
                    let selected = selected_ids.iter().any(|id| id == &part.id);
                    let part_id = part.id.clone();
                    let keyboard_id = part_id.clone();
                    let click_scope = scope.clone();
                    let pointer_scope = scope.clone();
                    let keyboard_scope = scope.clone();
                    let token = snapshot.token;
                    let click_hit = on_part_hit;
                    let pointer_down = on_part_pointer_down;
                    let keyboard_hit = on_part_hit;
                    let click_generation = generation;
                    let keyboard_generation = generation;
                    let reference = part.reference.clone();
                    let click_id = part_id.clone();
                    let definition_name = definition.map(|definition| definition.name.as_str()).unwrap_or("Part definition unavailable");
                    let label = format!("{reference}, {definition_name}, X {:.2} Y {:.2}", pose.at.x, pose.at.y);
                    let courtyard_points = definition.map(|definition| definition.courtyard.as_slice()).unwrap_or_default();
                    let courtyard = polygon_points(courtyard_points);
                    let hit_bounds = courtyard_bounds(courtyard_points);
                    let generator_parameters = part.generator_parameters.clone();
                    rsx! {
                        g {
                            key: "part-{part_id}",
                            class: "cursor-grab",
                            style: "cursor: pointer",
                            transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation}) {side_transform}",
                            role: "button",
                            tabindex: "0",
                            "aria-pressed": "{selected}",
                            "aria-label": "{label}",
                            "data-part-id": "{part.id}",
                            onpointerdown: move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                if pointer.button() != 0 { return; }
                                // Stop Dioxus bubbling as well as the native event below.
                                // A deselected part starts no drag to guard the empty-space path.
                                event.stop_propagation();
                                let modifiers = event.data().modifiers();
                                pointer_down.call(PcbPartPointerDown {
                                    scope: pointer_scope.clone(),
                                    token,
                                    generation,
                                    part_id: part_id.clone(),
                                    pointer_id: i64::from(pointer.pointer_id()),
                                    client_x: pointer.client_x(),
                                    client_y: pointer.client_y(),
                                    additive: modifiers.ctrl() || modifiers.meta(),
                                    range: modifiers.shift(),
                                });
                                pointer.prevent_default();
                                pointer.stop_propagation();
                            },
                            onclick: move |event: MouseEvent| {
                                event.stop_propagation();
                                // Pointer selection is already applied on pointerdown.
                                // Capture can retarget this compatibility click to the SVG.
                                // Keep click-only accessibility activation, without toggling twice.
                                if event.data().try_as_web_event().is_some_and(|event| event.detail() != 0) {
                                    return;
                                }
                                let modifiers = event.data().modifiers();
                                click_hit.call(PcbPartHit {
                                    scope: click_scope.clone(),
                                    token,
                                    generation: click_generation,
                                    part_id: click_id.clone(),
                                    additive: modifiers.ctrl() || modifiers.meta(),
                                    range: modifiers.shift(),
                                });
                            },
                            onkeydown: move |event: KeyboardEvent| {
                                let key = event.data().key().to_string();
                                if key != "Enter" && key != " " { return; }
                                event.prevent_default();
                                event.stop_propagation();
                                let modifiers = event.data().modifiers();
                                keyboard_hit.call(PcbPartHit {
                                    scope: keyboard_scope.clone(),
                                    token,
                                    generation: keyboard_generation,
                                    part_id: keyboard_id.clone(),
                                    additive: modifiers.ctrl() || modifiers.meta(),
                                    range: modifiers.shift(),
                                });
                            },
                            if !courtyard.is_empty() && !hidden_layers.contains("Courtyards") {
                                polygon { points: "{courtyard}", "data-layer": "Courtyards", class: if selected { "fill-selected stroke-primary stroke-[0.8] cursor-grab" } else { "fill-geometry-body stroke-geometry-part stroke-[0.4] cursor-grab" } }
                            }
                            if let Some(definition) = definition {
                                if definition.generator.is_some() {
                                    FootprintGraphics {
                                        definition: definition.clone(),
                                        parameters: generator_parameters.clone(),
                                        hidden_layers: Some(hidden_layers.clone()),
                                        part_side: Some(part.side.clone()),
                                    }
                                }
                                for pad in &definition.pads {
                                    {
                                        let radius = match &pad.shape {
                                            PadShape::Circle | PadShape::Oval => pad.size.x.min(pad.size.y) / 2.0,
                                            PadShape::Roundrect => pad.size.x.min(pad.size.y) / 4.0,
                                            PadShape::Rect => 0.0,
                                        };
                                        let drill = pad.drill;
                                        let pad_rotation = pad.rotation.unwrap_or(0.0);
                                        rsx! {
                                            g {
                                                key: "pad-{part_id}-{pad.id}",
                                                transform: "translate({pad.at.x} {pad.at.y}) rotate({pad_rotation})",
                                                if super::pcb_layers::pad_is_visible(pad, &part.side, &hidden_layers) {
                                                    rect {
                                                        class: "fill-geometry-part stroke-none",
                                                        "data-layer": "Pads",
                                                        x: "{-pad.size.x / 2.0}",
                                                        y: "{-pad.size.y / 2.0}",
                                                        width: "{pad.size.x}",
                                                        height: "{pad.size.y}",
                                                        rx: "{radius}",
                                                    }
                                                }
                                                if let Some(drill) = drill.filter(|_| !hidden_layers.contains("Holes")) {
                                                    circle { class: "fill-canvas stroke-geometry-outline stroke-[0.1]", "data-layer": "Holes", r: "{drill / 2.0}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if !hidden_layers.contains("References") {
                                text { transform: "scale(1,-1)", text_anchor: "middle", class: "font-mono text-[1.8px] fill-geometry-label pointer-events-none", "data-layer": "References", x: "0", y: "-5.2", "{reference}" }
                            }
                            rect {
                                class: "fill-transparent stroke-none [pointer-events:all] cursor-grab",
                                style: "cursor: pointer",
                                x: "{hit_bounds.0}",
                                y: "{hit_bounds.1}",
                                width: "{hit_bounds.2}",
                                height: "{hit_bounds.3}",
                            }
                        }
                    }
                }
            }
            for module in snapshot.scene.module_scenes.iter().filter(|module| {
                document.modules.iter().any(|instance| instance.id == module.id && instance.host_board_id == scope.board_id)
            }) {
                super::pcb_module_footprints::ModuleSourceFootprints {
                    key: "{module.id}",
                    module_id: module.id.clone(),
                    snapshot: snapshot.clone(),
                    on_select: on_module_select,
                }
            }
            super::pcb_module_footprints::PcbFindingMarkers {
                snapshot: snapshot.clone(), board_id: scope.board_id.clone(),
            }
        }
    }
}

fn courtyard_bounds(points: &[Vec2]) -> (f64, f64, f64, f64) {
    let Some(first) = points.first() else {
        // Match React’s missing-definition hit affordance without drawing a
        // fabricated courtyard into the accepted scene.
        return (-4.0, -4.0, 8.0, 8.0);
    };
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    (min_x, min_y, max_x - min_x, max_y - min_y)
}
