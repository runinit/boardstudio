//! Private geometry for projecting a mirrored pair before Core commits the source matrix.
use boardstudio_core::model::{Layout, LayoutMirrorLink, Matrix, MatrixScene, Mirror, Vec2};

#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairProjection {
    pub left: Layout,
    pub right: Layout,
    pub matrix: Matrix,
    /// Canvas-only reflected preview. Core derives the committed partner from `matrix`.
    pub right_preview: Matrix,
    pub center: Vec2,
    pub axis_x: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairGeometryInput {
    pub left_name: String,
    pub right_name: String,
    pub gap_mm: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairIds {
    pub right_matrix_id: String,
    pub left_layout_id: String,
    pub right_layout_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairPreviewCell {
    pub row: u32,
    pub column: u32,
    pub center: Vec2,
    pub rotation: f64,
    pub size: Vec2,
}

/// Adapt Core's projected scene to the canvas preview, adding only the visual key bounds.
/// Cell locations and rotations always come from `CoreRequest::ProjectMatrices`.
pub fn preview_cells(scene: &MatrixScene, matrix: &Matrix) -> Vec<PairPreviewCell> {
    let edge_gap = matrix.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
    scene
        .cells
        .iter()
        .filter(|cell| cell.enabled)
        .map(|cell| PairPreviewCell {
            row: cell.row,
            column: cell.column,
            center: cell.pose.at,
            rotation: cell.pose.rotation,
            size: Vec2 {
                x: (matrix.pitch.x - edge_gap.x).max(1.0),
                y: (matrix.pitch.y - edge_gap.y).max(1.0),
            },
        })
        .collect()
}

/// Matches React `pairAt`: inset both halves from the shared axis by half the requested key-edge
/// gap plus half the pitch, minus half the matrix edge gap.
pub fn project_mirrored_pair(
    mut matrix: Matrix,
    ids: &MirroredPairIds,
    request: &MirroredPairGeometryInput,
    center: Vec2,
) -> Result<MirroredPairProjection, String> {
    let left_name = request.left_name.trim();
    let right_name = request.right_name.trim();
    if left_name.is_empty() || right_name.is_empty() || left_name == right_name {
        return Err("Choose two distinct layout names.".into());
    }
    if !request.gap_mm.is_finite() || request.gap_mm < 0.0 {
        return Err("The gap must be a finite non-negative distance.".into());
    }
    if !center.x.is_finite() || !center.y.is_finite() {
        return Err("The placement point must be finite.".into());
    }
    if matrix.id.is_empty()
        || matrix.board_id.as_deref().is_none_or(str::is_empty)
        || ids.right_matrix_id.is_empty()
        || ids.left_layout_id.is_empty()
        || ids.right_layout_id.is_empty()
        || ids.left_layout_id == ids.right_layout_id
        || matrix.id == ids.right_matrix_id
    {
        return Err("The mirrored pair needs distinct live layout and matrix identities.".into());
    }
    let edge_gap_x = matrix.edge_gap.map(|gap| gap.x).unwrap_or(1.0);
    let pitch_x = matrix.pitch.x;
    if !edge_gap_x.is_finite() || edge_gap_x < 0.0 || !pitch_x.is_finite() || pitch_x <= 0.0 {
        return Err("The matrix pitch and edge gap must be finite and valid.".into());
    }
    let inset = (request.gap_mm + pitch_x - edge_gap_x) / 2.0;
    if !inset.is_finite() {
        return Err("The mirrored pair placement is outside the supported range.".into());
    }

    let board_id = matrix.board_id.clone().expect("validated board identity");
    let left_layout = Layout {
        id: ids.left_layout_id.clone(),
        name: left_name.to_owned(),
        board_id: board_id.clone(),
        matrix_id: matrix.id.clone(),
        part_ids: Vec::new(),
        mirror_link: None,
    };
    let right_layout = Layout {
        id: ids.right_layout_id.clone(),
        name: right_name.to_owned(),
        board_id,
        matrix_id: ids.right_matrix_id.clone(),
        part_ids: Vec::new(),
        mirror_link: Some(LayoutMirrorLink {
            source_id: left_layout.id.clone(),
            axis_x: center.x,
        }),
    };

    matrix.name = Some(left_name.to_owned());
    matrix.mirror = Some(Mirror::X);
    matrix.origin = Vec2 {
        x: center.x - inset,
        y: center.y,
    };
    let mut right_preview = matrix.clone();
    right_preview.id = ids.right_matrix_id.clone();
    right_preview.name = Some(right_name.to_owned());
    right_preview.mirror = Some(Mirror::None);
    right_preview.origin.x = center.x + inset;

    Ok(MirroredPairProjection {
        left: left_layout,
        right: right_layout,
        matrix,
        right_preview,
        center,
        axis_x: center.x,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::{
        CoreEngine,
        model::{CoreReply, CoreRequest, Mirror},
    };

    fn matrix() -> Matrix {
        serde_json::from_value(serde_json::json!({
            "id": "left-matrix",
            "name": null,
            "rows": 3,
            "columns": 5,
            "pitch": { "x": 19.05, "y": 19.05 },
            "origin": { "x": 0.0, "y": 0.0 },
            "definitionId": "preset/fixture",
            "partIds": [],
            "boardId": "board",
            "edgeGap": { "x": 1.0, "y": 1.0 },
            "rowOffsets": [{ "x": 2.0, "y": 3.0 }, { "x": -1.0, "y": 4.0 }, { "x": 0.0, "y": -2.0 }],
            "columnOffsets": [{ "x": 0.0, "y": 0.0 }, { "x": 1.5, "y": -0.5 }, { "x": 0.0, "y": 0.0 }],
            "columnStaggers": [0.5, 1.25],
            "columnSplays": [0.0, 12.0],
            "columnOrigins": [null, { "x": 20.0, "y": 3.0 }],
            "rotation": 8.0,
            "cells": [
                { "row": 0, "column": 1, "enabled": true, "offset": { "x": 0.25, "y": 0.75 }, "rotation": 2.0, "assemblies": [] },
                { "row": 1, "column": 3, "enabled": false, "assemblies": [] }
            ]
        }))
        .unwrap()
    }

    fn core_project(matrix: Matrix) -> MatrixScene {
        let mut core = CoreEngine::new();
        match core.handle(CoreRequest::ProjectMatrices {
            id: "pair-preview".into(),
            base_revision: 0,
            matrices: vec![matrix],
        }) {
            CoreReply::MatrixProjections { matrix_scenes, .. } => matrix_scenes
                .into_iter()
                .next()
                .expect("Core returns one matrix projection"),
            reply => panic!("Core did not project the preview matrix: {reply:?}"),
        }
    }

    fn request() -> MirroredPairGeometryInput {
        MirroredPairGeometryInput {
            left_name: " Left half ".into(),
            right_name: "Right half".into(),
            gap_mm: 24.0,
        }
    }

    fn ids() -> MirroredPairIds {
        MirroredPairIds {
            right_matrix_id: "right-matrix".into(),
            left_layout_id: "left-layout".into(),
            right_layout_id: "right-layout".into(),
        }
    }

    #[test]
    fn projection_matches_react_pair_at_and_leaves_reflection_to_core() {
        let pair = project_mirrored_pair(matrix(), &ids(), &request(), Vec2 { x: 100.0, y: 50.0 })
            .unwrap();

        assert_eq!(pair.left.name, "Left half");
        assert_eq!(pair.left.matrix_id, "left-matrix");
        assert_eq!(pair.right.matrix_id, "right-matrix");
        assert_eq!(
            pair.right.mirror_link.as_ref().unwrap().source_id,
            "left-layout"
        );
        assert_eq!(pair.right.mirror_link.as_ref().unwrap().axis_x, 100.0);
        assert_eq!(pair.matrix.mirror, Some(Mirror::X));
        assert_eq!(pair.right_preview.mirror, Some(Mirror::None));
        assert_eq!(pair.matrix.name.as_deref(), Some("Left half"));
        assert_eq!(pair.right_preview.name.as_deref(), Some("Right half"));
        assert!((pair.matrix.origin.x - 78.975).abs() < 1e-9);
        assert!((pair.right_preview.origin.x - 121.025).abs() < 1e-9);
        assert_eq!(pair.matrix.origin.y, 50.0);
        assert_eq!(pair.right_preview.origin.y, 50.0);
        assert_eq!(pair.axis_x, 100.0);
    }

    #[test]
    fn projection_rejects_invalid_names_gap_point_and_colliding_ids() {
        let mut invalid_request = request();
        invalid_request.right_name = " Left half ".into();
        assert!(
            project_mirrored_pair(matrix(), &ids(), &invalid_request, Vec2 { x: 0.0, y: 0.0 })
                .is_err()
        );

        let mut invalid_request = request();
        invalid_request.gap_mm = f64::NAN;
        assert!(
            project_mirrored_pair(matrix(), &ids(), &invalid_request, Vec2 { x: 0.0, y: 0.0 })
                .is_err()
        );

        assert!(
            project_mirrored_pair(
                matrix(),
                &ids(),
                &request(),
                Vec2 {
                    x: f64::INFINITY,
                    y: 0.0
                }
            )
            .is_err()
        );

        let mut colliding_ids = ids();
        colliding_ids.right_matrix_id = "left-matrix".into();
        assert!(
            project_mirrored_pair(
                matrix(),
                &colliding_ids,
                &request(),
                Vec2 { x: 0.0, y: 0.0 }
            )
            .is_err()
        );
    }

    #[test]
    fn canvas_preview_uses_core_projected_cell_poses_for_both_mirrored_halves() {
        let projection = project_mirrored_pair(
            matrix(),
            &ids(),
            &MirroredPairGeometryInput {
                left_name: "Left".into(),
                right_name: "Right".into(),
                gap_mm: 24.0,
            },
            Vec2::default(),
        )
        .unwrap();
        let left_scene = core_project(projection.matrix.clone());
        let right_scene = core_project(projection.right_preview.clone());
        let left = preview_cells(&left_scene, &projection.matrix);
        let right = preview_cells(&right_scene, &projection.right_preview);
        assert_eq!(left.len(), right.len());
        assert!(left.len() < (projection.matrix.rows * projection.matrix.columns) as usize);
        for (preview, projected) in left
            .iter()
            .zip(left_scene.cells.iter().filter(|cell| cell.enabled))
        {
            assert_eq!(preview.center, projected.pose.at);
            assert_eq!(preview.rotation, projected.pose.rotation);
        }
        for (preview, projected) in right
            .iter()
            .zip(right_scene.cells.iter().filter(|cell| cell.enabled))
        {
            assert_eq!(preview.center, projected.pose.at);
            assert_eq!(preview.rotation, projected.pose.rotation);
        }
        assert!(
            left_scene
                .cells
                .iter()
                .any(|cell| { cell.row == 0 && cell.column == 1 && cell.pose.rotation != 0.0 })
        );
        assert!(
            right_scene
                .cells
                .iter()
                .any(|cell| { cell.row == 0 && cell.column == 1 && cell.pose.rotation != 0.0 })
        );
    }
}
