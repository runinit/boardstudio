//! Private geometry for projecting a mirrored pair before Core commits the source matrix.
use boardstudio_application::{Durability, SnapshotToken};
use boardstudio_core::model::{Layout, LayoutMirrorLink, Matrix, Mirror, Vec2};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MirroredPairProjection {
    pub left: Layout,
    pub right: Layout,
    pub matrix: Matrix,
    /// Canvas-only reflected preview. Core derives the committed partner from `matrix`.
    pub right_preview: Matrix,
    pub center: Vec2,
    pub axis_x: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MirroredPairGeometryInput {
    pub left_name: String,
    pub right_name: String,
    pub gap_mm: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MirroredPairIds {
    pub right_matrix_id: String,
    pub left_layout_id: String,
    pub right_layout_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PairPreviewCell {
    pub row: u32,
    pub column: u32,
    pub center: Vec2,
    pub rotation: f64,
    pub size: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PairSnapshotIdentity {
    pub base_token: SnapshotToken,
    pub base_revision: u64,
    pub result_token: SnapshotToken,
    pub result_revision: u64,
    pub accepted_token: SnapshotToken,
    pub accepted_revision: u64,
}

pub(crate) fn result_snapshot_is_current(
    identity: PairSnapshotIdentity,
    ready: bool,
    durability: &Durability,
) -> bool {
    identity.result_token != identity.base_token
        && identity.result_revision > identity.base_revision
        && identity.accepted_token == identity.result_token
        && identity.accepted_revision == identity.result_revision
        && ready
        && *durability
            == (Durability::Saved {
                revision: identity.result_revision,
            })
}

/// Build preview-only cell poses for the newly prepared, unmodified matrix. These poses mirror
/// Core's fresh-matrix projection; this helper intentionally does not project live edits.
pub(crate) fn preview_cells(matrix: &Matrix) -> Vec<PairPreviewCell> {
    let mut cells = Vec::with_capacity((matrix.rows * matrix.columns) as usize);
    for row in 0..matrix.rows {
        for column in 0..matrix.columns {
            let cell = matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column);
            if cell.is_some_and(|cell| !cell.enabled) {
                continue;
            }
            let mut x = f64::from(column) * matrix.pitch.x
                + matrix
                    .row_offsets
                    .get(row as usize)
                    .copied()
                    .unwrap_or_default()
                    .x
                + matrix
                    .column_offsets
                    .get(column as usize)
                    .copied()
                    .unwrap_or_default()
                    .x
                + cell.and_then(|cell| cell.offset).unwrap_or_default().x;
            let mut y = f64::from(row) * matrix.pitch.y
                + matrix
                    .row_offsets
                    .get(row as usize)
                    .copied()
                    .unwrap_or_default()
                    .y
                + matrix
                    .column_offsets
                    .get(column as usize)
                    .copied()
                    .unwrap_or_default()
                    .y
                + cell.and_then(|cell| cell.offset).unwrap_or_default().y
                + matrix
                    .column_staggers
                    .iter()
                    .take(column as usize + 1)
                    .sum::<f64>();
            for index in (0..(column as usize + 1).min(matrix.column_splays.len())).rev() {
                let angle = matrix
                    .column_splays
                    .get(index)
                    .copied()
                    .unwrap_or(0.0)
                    .to_radians();
                if angle == 0.0 {
                    continue;
                }
                let pivot = matrix
                    .column_origins
                    .get(index)
                    .copied()
                    .flatten()
                    .unwrap_or(Vec2 {
                        x: index as f64 * matrix.pitch.x,
                        y: matrix.column_staggers.iter().take(index + 1).sum::<f64>(),
                    });
                let (sin, cos) = angle.sin_cos();
                let dx = x - pivot.x;
                let dy = y - pivot.y;
                x = pivot.x + dx * cos - dy * sin;
                y = pivot.y + dx * sin + dy * cos;
            }
            if matrix.mirror == Some(Mirror::X) {
                x = -x;
            }
            if matrix.mirror == Some(Mirror::Y) {
                y = -y;
            }
            let rotation = matrix.rotation.unwrap_or(0.0).to_radians();
            let (sin, cos) = rotation.sin_cos();
            let edge_gap = matrix.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
            cells.push(PairPreviewCell {
                row,
                column,
                center: Vec2 {
                    x: matrix.origin.x + x * cos - y * sin,
                    y: matrix.origin.y + x * sin + y * cos,
                },
                rotation: matrix.rotation.unwrap_or(0.0)
                    + matrix
                        .column_splays
                        .iter()
                        .take(column as usize + 1)
                        .sum::<f64>()
                        * if matches!(matrix.mirror, Some(Mirror::X | Mirror::Y)) {
                            -1.0
                        } else {
                            1.0
                        }
                    + cell.and_then(|cell| cell.rotation).unwrap_or(0.0),
                size: Vec2 {
                    x: (matrix.pitch.x - edge_gap.x).max(1.0),
                    y: (matrix.pitch.y - edge_gap.y).max(1.0),
                },
            });
        }
    }
    cells
}

/// Matches React `pairAt`: inset both halves from the shared axis by half the requested key-edge
/// gap plus half the pitch, minus half the matrix edge gap.
pub(crate) fn project_mirrored_pair(
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
    use boardstudio_core::model::Mirror;

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
            "cells": []
        }))
        .unwrap()
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
    fn preview_cells_stay_reflected_about_the_created_link_axis() {
        let projection = project_mirrored_pair(
            matrix(),
            &ids(),
            &MirroredPairGeometryInput {
                left_name: "Left".into(),
                right_name: "Right".into(),
                gap_mm: 24.0,
            },
            Vec2 { x: 12.0, y: -8.0 },
        )
        .unwrap();
        let left = preview_cells(&projection.matrix);
        let right = preview_cells(&projection.right_preview);
        assert_eq!(left.len(), right.len());
        for (left_cell, right_cell) in left.iter().zip(&right) {
            assert_eq!(
                (left_cell.row, left_cell.column),
                (right_cell.row, right_cell.column)
            );
            assert!(
                (left_cell.center.x + right_cell.center.x - 2.0 * projection.axis_x).abs() < 1e-9
            );
            assert!((left_cell.center.y - right_cell.center.y).abs() < 1e-9);
        }
    }

    #[test]
    fn created_pair_selection_requires_exact_advanced_saved_result() {
        let base = SnapshotToken(4);
        let result = SnapshotToken(5);
        let saved = Durability::Saved { revision: 12 };
        assert!(result_snapshot_is_current(
            PairSnapshotIdentity {
                base_token: base,
                base_revision: 11,
                result_token: result,
                result_revision: 12,
                accepted_token: result,
                accepted_revision: 12,
            },
            true,
            &saved
        ));
        assert!(!result_snapshot_is_current(
            PairSnapshotIdentity {
                base_token: base,
                base_revision: 11,
                result_token: result,
                result_revision: 12,
                accepted_token: base,
                accepted_revision: 11,
            },
            true,
            &saved
        ));
        assert!(!result_snapshot_is_current(
            PairSnapshotIdentity {
                base_token: base,
                base_revision: 11,
                result_token: base,
                result_revision: 12,
                accepted_token: base,
                accepted_revision: 12,
            },
            true,
            &saved
        ));
        assert!(!result_snapshot_is_current(
            PairSnapshotIdentity {
                base_token: base,
                base_revision: 11,
                result_token: result,
                result_revision: 11,
                accepted_token: result,
                accepted_revision: 11,
            },
            true,
            &saved
        ));
        assert!(!result_snapshot_is_current(
            PairSnapshotIdentity {
                base_token: base,
                base_revision: 11,
                result_token: result,
                result_revision: 12,
                accepted_token: result,
                accepted_revision: 12,
            },
            false,
            &saved
        ));
        assert!(!result_snapshot_is_current(
            PairSnapshotIdentity {
                base_token: base,
                base_revision: 11,
                result_token: result,
                result_revision: 12,
                accepted_token: result,
                accepted_revision: 12,
            },
            true,
            &Durability::Saving { revision: 12 },
        ));
    }
}
