//! Whole-matrix drag solving: Origin and rotation through the anchor frame and Follow basis.

use boardstudio_model::model::{
    Follow, Matrix, MatrixAnchor, MatrixScene, MirroredItemKind, Pose2, ProjectDoc, Vec2,
};

/// Where a matrix drag's Follow permission is evaluated: the matrix's own
/// world axes, or the source-world axes transported through the pair map for
/// a linked partner. Mirrors the kernel's anchor evaluation basis from the
/// accepted document; Core remains the placement authority and re-validates
/// the solved Origin on commit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatrixDragTransport {
    World,
    Reflected { translation: Vec2, rotation: f64 },
}

/// The Follow basis for dragging a matrix, derived from the accepted document.
pub fn matrix_drag_transport(document: &ProjectDoc, matrix_id: &str) -> MatrixDragTransport {
    for layout in &document.layouts {
        if let Some(link) = &layout.mirror_link
            && link.correspondences.iter().any(|record| {
                record.kind == MirroredItemKind::Matrix && record.mirror_id == matrix_id
            })
        {
            return MatrixDragTransport::Reflected {
                translation: Vec2 {
                    x: 2.0 * link.axis_x,
                    y: 0.0,
                },
                rotation: 0.0,
            };
        }
    }
    if let Some((derived, _, _)) =
        boardstudio_model::mirrored_half::derived_matrix(document, matrix_id)
        && let Some(pair) = document
            .hardware
            .iter()
            .flat_map(|hardware| &hardware.pairs)
            .find(|pair| pair.id == derived.id)
    {
        return MatrixDragTransport::Reflected {
            translation: pair.mapping.at,
            rotation: pair.mapping.rotation,
        };
    }
    MatrixDragTransport::World
}

/// The anchor context a matrix drag inverse-solves through: the anchor key's
/// final world pose, the Follow subset and the transported basis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MatrixDragAnchor {
    pub key_pose: Pose2,
    pub follow: Follow,
    pub transport: MatrixDragTransport,
}

/// The anchor key's final world pose from the accepted scene, or `None` when
/// the anchor target is not laid out and the drag cannot start.
pub fn matrix_drag_anchor_pose(scenes: &[MatrixScene], anchor: &MatrixAnchor) -> Option<Pose2> {
    crate::scene_cell(scenes, &anchor.matrix_id, anchor.row, anchor.column).map(|cell| cell.pose)
}

fn reflect_delta(transport: MatrixDragTransport, delta: Vec2) -> Vec2 {
    match transport {
        MatrixDragTransport::World => delta,
        MatrixDragTransport::Reflected { rotation, .. } => {
            let (sin, cos) = rotation.to_radians().sin_cos();
            Vec2 {
                x: -delta.x * cos - delta.y * sin,
                y: -delta.x * sin + delta.y * cos,
            }
        }
    }
}

fn zero_unfollowed(follow: &Follow, point: Vec2) -> Vec2 {
    Vec2 {
        x: if follow.x { point.x } else { 0.0 },
        y: if follow.y { point.y } else { 0.0 },
    }
}

/// Solve the dragged matrix's new Origin from a world drag delta. Unanchored
/// matrices move 1:1; anchored matrices inverse-solve through the anchor
/// frame with the delta constrained to the Follow directions, so an
/// unfollowed component never moves the frame. Components Follow discards
/// keep their authored values, so a drag outside every Follow direction is a
/// no-op that commits nothing.
pub fn solve_matrix_origin(
    matrix: &Matrix,
    anchor: Option<MatrixDragAnchor>,
    world_delta: Vec2,
) -> Vec2 {
    let constrained = match anchor {
        None => world_delta,
        Some(anchor) => reflect_delta(
            anchor.transport,
            zero_unfollowed(&anchor.follow, reflect_delta(anchor.transport, world_delta)),
        ),
    };
    // Through the inverse of the anchor frame for anchored matrices; the
    // identity for unanchored ones, whose Origin is already in world axes.
    let (sin, cos) = anchor
        .map(|anchor| anchor.key_pose.rotation.to_radians().sin_cos())
        .unwrap_or((0.0, 1.0));
    Vec2 {
        x: matrix.origin.x + constrained.x * cos + constrained.y * sin,
        y: matrix.origin.y - constrained.x * sin + constrained.y * cos,
    }
}

/// Solve the dragged matrix's new stored rotation, or `None` when rotation is
/// not followed and the Rotate gesture must stay disabled.
pub fn solve_matrix_rotation(
    rotation: f64,
    follow_rotation: bool,
    delta_degrees: f64,
) -> Option<f64> {
    follow_rotation.then_some(rotation + delta_degrees)
}
