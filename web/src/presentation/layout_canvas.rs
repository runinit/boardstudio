//! Read-only Layout canvas projection and scene rendering.
use super::{FootprintGraphics, layout_matrix_relations};
use boardstudio_core::model::{
    Matrix, MatrixScene, Part, PartDefinition, PartKind, Pose2, Side, Transform, Vec2,
};
use dioxus::prelude::*;
use std::collections::BTreeMap;

pub(super) struct MatrixPointerDown {
    pub(super) event: PointerEvent,
    pub(super) matrix_id: String,
    pub(super) row: u32,
    pub(super) column: u32,
    pub(super) part_id: Option<String>,
}

pub(super) struct PartPointerDown {
    pub(super) event: PointerEvent,
    pub(super) part_id: String,
}

pub(super) struct LayoutCanvasInput<'a> {
    pub(super) visible: &'a [Part],
    pub(super) definitions: &'a BTreeMap<&'a str, &'a PartDefinition>,
    pub(super) matrices: &'a [&'a Matrix],
    pub(super) matrix_scenes: &'a BTreeMap<&'a str, &'a MatrixScene>,
    pub(super) transforms: &'a [Transform],
    pub(super) selected_ids: &'a [String],
    pub(super) selected_context: Option<&'a super::objects::ScopedTreeContext>,
    pub(super) keys_visible: bool,
    pub(super) components_visible: bool,
    pub(super) keycaps_visible: bool,
    pub(super) footprints_visible: bool,
    pub(super) on_matrix_pointer_down: EventHandler<MatrixPointerDown>,
    pub(super) on_part_pointer_down: EventHandler<PartPointerDown>,
}

struct Projection<'a> {
    visible_by_id: BTreeMap<&'a str, &'a Part>,
    matrix_keycaps: BTreeMap<&'a str, Vec2>,
    part_transforms: BTreeMap<&'a str, Pose2>,
}

fn project<'a>(
    visible: &'a [Part],
    matrices: &'a [&'a Matrix],
    matrix_scenes: &'a BTreeMap<&'a str, &'a MatrixScene>,
    transforms: &'a [Transform],
) -> Projection<'a> {
    let visible_by_id = visible
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();
    let mut matrix_keycaps = BTreeMap::new();
    for matrix in matrices {
        if let Some(projected) = matrix_scenes.get(matrix.id.as_str()) {
            for cell in projected.cells.iter().filter(|cell| cell.enabled) {
                if let Some(part_id) = cell.member_id.as_deref() {
                    matrix_keycaps
                        .entry(part_id)
                        .or_insert_with(|| resolved_matrix_keycap(None, None, matrix));
                }
            }
        }
    }
    let mut part_transforms = BTreeMap::new();
    for transform in transforms {
        part_transforms
            .entry(transform.id.as_str())
            .or_insert(transform.pose);
    }
    Projection {
        visible_by_id,
        matrix_keycaps,
        part_transforms,
    }
}

pub(super) fn scene(input: LayoutCanvasInput<'_>) -> Element {
    let projection = project(
        input.visible,
        input.matrices,
        input.matrix_scenes,
        input.transforms,
    );
    rsx! {
        if input.keys_visible {
            for matrix in input.matrices {
                if let Some(projected) = input.matrix_scenes.get(matrix.id.as_str()) {
                    for cell in projected.cells.iter().filter(|cell| cell.enabled) {
                        {
                            let member = cell
                                .member_id
                                .as_deref()
                                .and_then(|id| projection.visible_by_id.get(id).copied());
                            let definition = match member {
                                Some(part) => input.definitions.get(part.definition_id.as_str()).copied(),
                                None => input.definitions.get(matrix.definition_id.as_str()).copied(),
                            };
                            let size = resolved_matrix_keycap(member, definition, matrix);
                            let selected = cell
                                .member_id
                                .as_ref()
                                .is_some_and(|id| input.selected_ids.contains(id))
                                || input.selected_context.is_some_and(|selected| {
                                    matches!(
                                        &selected.context,
                                        super::objects::TreeContext::Key { matrix_id, row, column }
                                            if matrix_id == &matrix.id && *row == cell.row && *column == cell.column
                                    )
                                });
                            let on_pointer_down = input.on_matrix_pointer_down;
                            let matrix_id = matrix.id.clone();
                            let row = cell.row;
                            let column = cell.column;
                            let part_id = cell.member_id.clone();
                            rsx! {
                                layout_matrix_relations::RelationMatrixKey {
                                    key: "{matrix.id}-{cell.row}-{cell.column}",
                                    part_id: cell.member_id.clone(),
                                    selected,
                                    x: -size.x / 2.0,
                                    y: -size.y / 2.0,
                                    width: size.x,
                                    height: size.y,
                                    transform: format!("translate({} {}) rotate({})", cell.pose.at.x, cell.pose.at.y, cell.pose.rotation),
                                    matrix_id: matrix.id.clone(),
                                    row: cell.row,
                                    column: cell.column,
                                    on_pointer_down: move |event: PointerEvent| {
                                        on_pointer_down.call(MatrixPointerDown {
                                            event,
                                            matrix_id: matrix_id.clone(),
                                            row,
                                            column,
                                            part_id: part_id.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for part in input.visible {
            {
                let pose = projection
                    .part_transforms
                    .get(part.id.as_str())
                    .copied()
                    .unwrap_or(part.pose);
                let definition = input.definitions.get(part.definition_id.as_str()).copied();
                let courtyard = definition.map(|definition| polygon_points(&definition.courtyard)).unwrap_or_default();
                let selected = input.selected_ids.contains(&part.id);
                let side_transform = if matches!(part.side, Side::Back) { "scale(-1 1)" } else { "" };
                let is_matrix_key = projection.matrix_keycaps.contains_key(part.id.as_str());
                let is_switch = is_matrix_key || definition.is_some_and(|definition| matches!(definition.kind, PartKind::Switch));
                let layer_visible = if is_switch { input.keys_visible } else { input.components_visible };
                let is_encoder = definition.is_some_and(|definition| matches!(definition.kind, PartKind::Encoder) || definition.input_profile.as_ref().is_some_and(|profile| profile.rotary.is_some()));
                let standalone_switch = definition.is_some_and(|definition| matches!(definition.kind, PartKind::Switch));
                let keycap = if is_encoder { None } else if is_matrix_key || standalone_switch {
                    part.keycap
                        .or_else(|| definition.and_then(|definition| definition.keycap))
                        .or_else(|| projection.matrix_keycaps.get(part.id.as_str()).copied())
                } else {
                    None
                };
                let show_keycap = keycap.filter(|_| input.keycaps_visible);
                let footprints_on = input.footprints_visible;
                let on_pointer_down = input.on_part_pointer_down;
                let part_id = part.id.clone();
                rsx! {
                    if layer_visible || show_keycap.is_some() {
                        g {
                            key: "{part.id}",
                            class: "m1-scene-part",
                            transform: "translate({pose.at.x},{pose.at.y}) rotate({pose.rotation}) {side_transform}",
                            "data-part-id": "{part.id}",
                            onpointerdown: move |event: PointerEvent| {
                                on_pointer_down.call(PartPointerDown { event, part_id: part_id.clone() });
                            },
                            if layer_visible {
                                layout_matrix_relations::RelationPartOutline {
                                    key: "{part.id}", part_id: part.id.clone(), selected, points: courtyard
                                }
                            }
                            if layer_visible && footprints_on {
                                if let Some(definition) = definition {
                                    FootprintGraphics { definition: definition.clone(), parameters: part.generator_parameters.clone() }
                                    for pad in &definition.pads {
                                        {
                                            let rx = match &pad.shape {
                                                boardstudio_core::model::PadShape::Circle | boardstudio_core::model::PadShape::Oval => pad.size.x.min(pad.size.y) / 2.0,
                                                boardstudio_core::model::PadShape::Roundrect => pad.size.x.min(pad.size.y) / 4.0,
                                                boardstudio_core::model::PadShape::Rect => 0.0,
                                            };
                                            rsx! {
                                                g { transform: "translate({pad.at.x} {pad.at.y}) rotate({pad.rotation.unwrap_or(0.0)})",
                                                    if pad.plated != Some(false) {
                                                        rect { class: "m1-part-pad", x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}", rx: "{rx}" }
                                                    }
                                                    if let Some(drill) = pad.drill { circle { class: "m1-part-drill", r: "{drill / 2.0}" } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(size) = show_keycap {
                                {
                                    let inset = 1.5_f64.min(size.x / 6.0).min(size.y / 6.0);
                                    rsx! {
                                        g { class: if selected { "m1-keycap-overlay is-selected" } else { "m1-keycap-overlay" }, "aria-hidden": "true",
                                            rect { x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}", rx: "0.9" }
                                            rect { class: "m1-keycap-top", x: "{-size.x / 2.0 + inset}", y: "{-size.y / 2.0 + inset}", width: "{size.x - inset * 2.0}", height: "{size.y - inset * 2.0}", rx: "0.7" }
                                        }
                                    }
                                }
                            }
                            if let Some(size) = show_keycap {
                                rect { class: "m1-part-hit-area", x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}" }
                            }
                            if layer_visible && show_keycap.is_none() {
                                text { transform: "scale(1,-1)", text_anchor: "middle", class: "m1-part-label", x: "0", y: "-5.2", "{part.reference}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn resolved_matrix_keycap(
    part: Option<&Part>,
    definition: Option<&PartDefinition>,
    matrix: &Matrix,
) -> Vec2 {
    part.and_then(|part| part.keycap)
        .or_else(|| definition.and_then(|definition| definition.keycap))
        .unwrap_or(Vec2 {
            x: (matrix.pitch.x - matrix.edge_gap.map(|gap| gap.x).unwrap_or(1.0)).max(1.0),
            y: (matrix.pitch.y - matrix.edge_gap.map(|gap| gap.y).unwrap_or(1.0)).max(1.0),
        })
}

fn polygon_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{MatrixSceneCell, Transform};
    use serde_json::json;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn projection_indexes_visible_parts_enabled_matrix_keycaps_and_first_transform() {
        let visible: Vec<Part> = vec![
            serde_json::from_value(json!({
                "id": "key-1", "definitionId": "switch", "reference": "SW1",
                "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front"
            }))
            .unwrap(),
        ];
        let matrix: Matrix = serde_json::from_value(json!({
            "id": "matrix", "rows": 1, "columns": 1, "pitch": {"x": 19.0, "y": 20.0},
            "origin": {"x": 0.0, "y": 0.0}, "definitionId": "switch", "partIds": ["key-1"],
            "boardId": "board", "edgeGap": {"x": 2.0, "y": 3.0}, "cells": []
        }))
        .unwrap();
        let matrix_scene = MatrixScene {
            matrix_id: "matrix".into(),
            cells: vec![
                MatrixSceneCell {
                    row: 0,
                    column: 0,
                    enabled: true,
                    member_id: Some("key-1".into()),
                    pose: Pose2 {
                        at: Vec2 { x: 0.0, y: 0.0 },
                        rotation: 0.0,
                    },
                },
                MatrixSceneCell {
                    row: 0,
                    column: 1,
                    enabled: false,
                    member_id: Some("disabled".into()),
                    pose: Pose2 {
                        at: Vec2 { x: 0.0, y: 0.0 },
                        rotation: 0.0,
                    },
                },
            ],
            columns: vec![],
            companions: vec![],
        };
        let matrix_scenes = BTreeMap::from([("matrix", &matrix_scene)]);
        let matrices = [&matrix];
        let transforms = [
            Transform {
                id: "key-1".into(),
                pose: Pose2 {
                    at: Vec2 { x: 5.0, y: 6.0 },
                    rotation: 7.0,
                },
            },
            Transform {
                id: "key-1".into(),
                pose: Pose2 {
                    at: Vec2 { x: 8.0, y: 9.0 },
                    rotation: 10.0,
                },
            },
        ];
        let projection = project(&visible, &matrices, &matrix_scenes, &transforms);

        assert!(projection.visible_by_id.contains_key("key-1"));
        assert_eq!(
            projection.matrix_keycaps.get("key-1"),
            Some(&Vec2 { x: 17.0, y: 17.0 })
        );
        assert!(!projection.matrix_keycaps.contains_key("disabled"));
        assert_eq!(
            projection.part_transforms.get("key-1").unwrap().at,
            Vec2 { x: 5.0, y: 6.0 }
        );
    }
}
