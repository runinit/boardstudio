//! Read-only host scene projection for the currently accepted PCB board.
use crate::presentation::footprint_graphics::FootprintGraphics;
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::model::{PadShape, Side, Vec2};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct PcbPartHit {
    pub(in crate::presentation) scope: Scope,
    pub(in crate::presentation) token: SnapshotToken,
    pub(in crate::presentation) generation: u64,
    pub(in crate::presentation) part_id: String,
    pub(in crate::presentation) additive: bool,
    pub(in crate::presentation) range: bool,
}

#[derive(Props, Clone, PartialEq)]
pub(in crate::presentation) struct PcbSceneProps {
    pub(in crate::presentation) snapshot: AcceptedSnapshot,
    pub(in crate::presentation) scope: Scope,
    pub(in crate::presentation) selected_ids: Vec<String>,
    pub(in crate::presentation) generation: u64,
    pub(in crate::presentation) on_part_hit: EventHandler<PcbPartHit>,
}

#[component]
pub(in crate::presentation) fn PcbScene(props: PcbSceneProps) -> Element {
    let snapshot = props.snapshot;
    let scope = props.scope;
    let selected_ids = props.selected_ids;
    let on_part_hit = props.on_part_hit;
    let generation = props.generation;

    if snapshot.session_epoch != scope.session_epoch || snapshot.document.id != scope.document_id {
        return rsx! {
            g { class: "m1-pcb-scene-state", "data-scene-status": "stale", role: "status" }
        };
    }

    let document = &snapshot.document;
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
    else {
        return rsx! {
            g { class: "m1-pcb-scene-state", "data-scene-status": "unavailable", role: "status" }
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
        g { class: "m1-pcb-scene", "data-board-id": board_id, "data-scene-status": "ready",
            for (index, contour) in contours.iter().enumerate() {
                polygon {
                    key: "outline-{index}",
                    points: points(&contour.points),
                    class: if contour.hole { "m1-outline is-hole" } else { "m1-outline" },
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
                    let keyboard_scope = scope.clone();
                    let token = snapshot.token;
                    let click_hit = on_part_hit;
                    let keyboard_hit = on_part_hit;
                    let click_generation = generation;
                    let keyboard_generation = generation;
                    let reference = part.reference.clone();
                    let click_id = part_id.clone();
                    let definition_name = definition.map(|definition| definition.name.as_str()).unwrap_or("Part definition unavailable");
                    let label = format!("{reference}, {definition_name}, X {:.2} Y {:.2}", pose.at.x, pose.at.y);
                    let courtyard_points = definition.map(|definition| definition.courtyard.as_slice()).unwrap_or_default();
                    let courtyard = points(courtyard_points);
                    let hit_bounds = courtyard_bounds(courtyard_points);
                    let generator_parameters = part.generator_parameters.clone();
                    rsx! {
                        g {
                            key: "part-{part_id}",
                            class: if selected { "m1-scene-part m1-pcb-part is-selected" } else { "m1-scene-part m1-pcb-part" },
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
                                pointer.stop_propagation();
                            },
                            onclick: move |event: MouseEvent| {
                                event.stop_propagation();
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
                            if !courtyard.is_empty() {
                                polygon { points: "{courtyard}", class: if selected { "m1-part selected" } else { "m1-part" } }
                            }
                            if let Some(definition) = definition {
                                if definition.generator.is_some() {
                                    FootprintGraphics { definition: definition.clone(), parameters: generator_parameters.clone() }
                                }
                                for pad in &definition.pads {
                                    {
                                        let radius = match &pad.shape {
                                            PadShape::Circle | PadShape::Oval => pad.size.x.min(pad.size.y) / 2.0,
                                            PadShape::Roundrect => pad.size.x.min(pad.size.y) / 4.0,
                                            PadShape::Rect => 0.0,
                                        };
                                        let plated = pad.plated != Some(false);
                                        let drill = pad.drill;
                                        let pad_rotation = pad.rotation.unwrap_or(0.0);
                                        rsx! {
                                            g {
                                                key: "pad-{part_id}-{pad.id}",
                                                transform: "translate({pad.at.x} {pad.at.y}) rotate({pad_rotation})",
                                                if plated {
                                                    rect {
                                                        class: "m1-part-pad",
                                                        x: "{-pad.size.x / 2.0}",
                                                        y: "{-pad.size.y / 2.0}",
                                                        width: "{pad.size.x}",
                                                        height: "{pad.size.y}",
                                                        rx: "{radius}",
                                                    }
                                                }
                                                if let Some(drill) = drill {
                                                    circle { class: "m1-part-drill", r: "{drill / 2.0}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            text { transform: "scale(1,-1)", text_anchor: "middle", class: "m1-part-label", x: "0", y: "-5.2", "{reference}" }
                            rect {
                                class: "m1-part-hit-area",
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
