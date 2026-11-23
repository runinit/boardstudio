//! Document indexes used by the Layout canvas.
use boardstudio_model::model::{Matrix, MatrixScene, Part, PartDefinition, Pose2, Transform, Vec2};
use std::collections::BTreeMap;

pub struct Projection<'a> {
    pub visible_by_id: BTreeMap<&'a str, &'a Part>,
    pub matrix_keycaps: BTreeMap<&'a str, Vec2>,
    pub part_transforms: BTreeMap<&'a str, Pose2>,
}

pub fn project<'a>(
    visible: &'a [&'a Part],
    matrices: &'a [&'a Matrix],
    matrix_scenes: &'a BTreeMap<&'a str, &'a MatrixScene>,
    transforms: &'a [Transform],
) -> Projection<'a> {
    let visible_by_id = visible
        .iter()
        .map(|part| (part.id.as_str(), *part))
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

pub fn resolved_matrix_keycap(
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::MatrixSceneCell;
    use serde_json::json;

    #[test]
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
                    shift_frame: Default::default(),
                    offset_frame: Default::default(),
                    row_shift: Vec2 { x: 0.0, y: 0.0 },
                    key_shift: Vec2 { x: 0.0, y: 0.0 },
                    row_bend: 0.0,
                    key_bend: 0.0,
                    cumulative_shift: Vec2 { x: 0.0, y: 0.0 },
                    cumulative_bend: 0.0,
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
                    shift_frame: Default::default(),
                    offset_frame: Default::default(),
                    row_shift: Vec2 { x: 0.0, y: 0.0 },
                    key_shift: Vec2 { x: 0.0, y: 0.0 },
                    row_bend: 0.0,
                    key_bend: 0.0,
                    cumulative_shift: Vec2 { x: 0.0, y: 0.0 },
                    cumulative_bend: 0.0,
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
        let visible_refs: Vec<_> = visible.iter().collect();
        let projection = project(&visible_refs, &matrices, &matrix_scenes, &transforms);

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
