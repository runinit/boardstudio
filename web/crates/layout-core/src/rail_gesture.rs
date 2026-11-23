//! Pure Layout rail gestures: every matrix and part handle drag maps one pointer sample to one
//! absolute `EditOperation` plus its popup text. The web overlay owns pointers, capture and
//! commits; nothing here touches Dioxus or the DOM, so snapping, sweeps and nudges run natively.
use crate::matrix_transform_operation::{
    ArrowKey, MatrixDragAnchor, PlacementParameter, PlacementValue, SceneBasis, column_angle,
    gesture_value, nudge_value, placement_operation, solve_matrix_origin, solve_matrix_rotation,
};
use crate::outline_snapping::{Origin, snap_origin};
use boardstudio_core_kernel::matrix::layout::evaluated_part_pose;
use boardstudio_model::model::{
    EditOperation, Matrix, MatrixSplayAffect, MatrixSplayChange, Mirror, PartAnchor,
    PartAnchorValueChange, ProjectDoc, Vec2,
};
use boardstudio_web_ui_model::transform_feedback::FeedbackDelta;

/// The value an anchored part drag writes: Spread and Stagger edit the values carried from the
/// anchor key, Offset and Rotation the part's local values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PartAxis {
    Spread,
    Stagger,
    Offset,
    Rotation,
}

/// What one handle drag edits, captured at pointer-down.
#[derive(Clone, Debug)]
pub enum RailGesture {
    /// A column or row placement parameter; `basis` is the frame the world delta projects onto.
    Placement {
        matrix: Matrix,
        parameter: PlacementParameter,
        basis: SceneBasis,
    },
    /// A column's splay angle, or with `swing` its origin carried without hold-still compensation.
    Splay {
        matrix: Matrix,
        column: u32,
        origin: Vec2,
        start_angle: f64,
        swing: bool,
        affect: MatrixSplayAffect,
        column_base: Vec2,
    },
    /// A column's splay origin point; `swing` skips the hold-still compensation.
    Origin {
        matrix: Matrix,
        column: u32,
        swing: bool,
        column_base: Vec2,
        start_origin: Vec2,
    },
    MatrixMove {
        matrix: Matrix,
        anchor: Option<MatrixDragAnchor>,
    },
    MatrixRotate {
        matrix: Matrix,
        anchor: Option<MatrixDragAnchor>,
        center: Vec2,
        start_angle: f64,
    },
    /// An anchored part's carried or local value.
    PartValue {
        part_id: String,
        axis: PartAxis,
        anchor: PartAnchor,
        base_rotation: f64,
        center: Vec2,
        start_angle: f64,
    },
    /// A detached part's rotation, swept around its centre.
    PartRotate {
        part_id: String,
        center: Vec2,
        start_angle: f64,
        start_rotation: f64,
    },
}

/// One pointer sample in world units.
pub struct RailInput<'a> {
    pub point: Vec2,
    /// World travel since pointer-down along the canvas axes (y up).
    pub world_delta: Vec2,
    pub alt: bool,
    /// The grid pitch a part drag snaps against (a matrix drag uses its own).
    pub pitch: Vec2,
    pub snap_fraction: f64,
    pub geometry_snap: bool,
    pub origins: &'a [Origin],
    /// World radius within which an origin drop snaps to part geometry.
    pub tolerance: f64,
}

/// The operation a sample proposes with its popup text; `guide` is a snap target to mark.
#[derive(Clone, Debug, PartialEq)]
pub struct RailPreview {
    pub operation: EditOperation,
    pub value: String,
    pub delta: FeedbackDelta,
    pub guide: Option<Vec2>,
}

/// The grid step: `fraction` of the pitch, or an absolute step in millimetres when negative.
pub fn grid_step(pitch: Vec2, fraction: f64) -> Option<Vec2> {
    if fraction == 0.0 {
        return None;
    }
    let step = |pitch: f64| {
        if fraction < 0.0 {
            -fraction
        } else {
            pitch * fraction
        }
        .max(0.001)
    };
    Some(Vec2 {
        x: step(pitch.x),
        y: step(pitch.y),
    })
}

/// The parameter's stored value; an absent Spread or Padding reads the pitch.
pub fn placement_current(matrix: &Matrix, parameter: PlacementParameter) -> PlacementValue {
    let column = |c: u32| matrix.column_layout.get(c as usize);
    let row = |r: u32| matrix.row_layout.get(r as usize);
    let cell = |r: u32, c: u32| {
        matrix
            .cells
            .iter()
            .find(|cell| cell.row == r && cell.column == c)
    };
    match parameter {
        PlacementParameter::Stagger { column: c } => {
            PlacementValue::Stagger(column(c).map_or(0.0, |x| x.stagger))
        }
        PlacementParameter::ColumnOffset { column: c } => {
            PlacementValue::Offset(column(c).map(|x| x.offset).unwrap_or_default())
        }
        PlacementParameter::RowOffset { row: r } => {
            PlacementValue::Offset(row(r).map(|x| x.offset).unwrap_or_default())
        }
        PlacementParameter::RowShift { row: r } => {
            PlacementValue::Shift(row(r).map(|x| x.shift).unwrap_or_default())
        }
        PlacementParameter::KeyShift { row: r, column: c } => {
            PlacementValue::Shift(cell(r, c).and_then(|x| x.shift).unwrap_or_default())
        }
        PlacementParameter::RowBend { row: r } => {
            PlacementValue::Bend(row(r).map_or(0.0, |x| x.bend))
        }
        PlacementParameter::KeyBend { row: r, column: c } => {
            PlacementValue::Bend(cell(r, c).and_then(|x| x.bend).unwrap_or(0.0))
        }
        PlacementParameter::ColumnSpread { column: c } => {
            PlacementValue::Spread(column(c).and_then(|x| x.spread).unwrap_or(matrix.pitch.x))
        }
        PlacementParameter::RowPadding { row: r } => {
            PlacementValue::Padding(row(r).and_then(|x| x.padding).unwrap_or(matrix.pitch.y))
        }
    }
}

/// The committed value text for a placement result ("Spread 28.57 mm" is the name plus this).
pub fn format_placement_value(value: PlacementValue) -> String {
    match value {
        PlacementValue::Stagger(v) | PlacementValue::Spread(v) | PlacementValue::Padding(v) => {
            format!("{v:.2} mm")
        }
        PlacementValue::Offset(v) | PlacementValue::Shift(v) => {
            format!("X {:.2}, Y {:.2} mm", v.x, v.y)
        }
        PlacementValue::Bend(v) => format!("{v:.1}°"),
    }
}

fn placement_delta(start: PlacementValue, end: PlacementValue) -> FeedbackDelta {
    use PlacementValue as V;
    match (start, end) {
        (V::Stagger(a), V::Stagger(b))
        | (V::Spread(a), V::Spread(b))
        | (V::Padding(a), V::Padding(b)) => FeedbackDelta::Mm(b - a),
        (V::Offset(a), V::Offset(b)) | (V::Shift(a), V::Shift(b)) => {
            FeedbackDelta::Mm2(b.x - a.x, b.y - a.y)
        }
        (V::Bend(a), V::Bend(b)) => FeedbackDelta::Deg(b - a),
        _ => FeedbackDelta::Text(String::new()),
    }
}

/// The carried (Spread, Stagger) axes follow the anchor key's rotation; the local Offset frame
/// adds the carried Splay, matching Core's anchor evaluation.
pub fn part_placement_basis(base_degrees: f64, splay_degrees: f64) -> SceneBasis {
    let base = base_degrees.to_radians();
    let frame = (base_degrees + splay_degrees).to_radians();
    let (x, y) = (
        Vec2 {
            x: frame.cos(),
            y: frame.sin(),
        },
        Vec2 {
            x: -frame.sin(),
            y: frame.cos(),
        },
    );
    SceneBasis {
        stagger_axis: Vec2 {
            x: -base.sin(),
            y: base.cos(),
        },
        axis_x: x,
        axis_y: y,
        shift_axis_x: x,
        shift_axis_y: y,
        spread_axis: Some(Vec2 {
            x: base.cos(),
            y: base.sin(),
        }),
    }
}

/// The follow-filtered rotation of an anchor key, or `None` when the key no longer resolves.
pub fn anchor_key_base_rotation(document: &ProjectDoc, anchor: &PartAnchor) -> Option<f64> {
    let id = format!(
        "matrix/{}/r{}c{}",
        anchor.matrix_id, anchor.row, anchor.column
    );
    let member = document.parts.iter().find(|part| part.id == id)?;
    Some(if anchor.follow.rotation {
        evaluated_part_pose(document, member).rotation
    } else {
        0.0
    })
}

/// Why a part rail edit has to stop: the part is gone, locked, a matrix member, or driven.
pub fn part_edit_guard(document: &ProjectDoc, part_id: &str, anchored: bool) -> Option<String> {
    let Some(part) = document.parts.iter().find(|part| part.id == part_id) else {
        return Some("The part no longer exists.".into());
    };
    if part.locked == Some(true) {
        return Some("This part is locked.".into());
    }
    if part.anchor.is_some() != anchored {
        return Some(
            if anchored {
                "The part is no longer anchored."
            } else {
                "The part is now anchored."
            }
            .into(),
        );
    }
    if document
        .matrices
        .iter()
        .any(|matrix| matrix.part_ids.contains(&part.id))
    {
        return Some("This part follows its matrix.".into());
    }
    match document.constraint_driving_part(part_id) {
        Ok(Some(_)) => Some("This part is driven by a relationship.".into()),
        Ok(None) => None,
        Err(_) => Some("The saved placement relationship could not be verified.".into()),
    }
}

fn sweep(center: Vec2, start_angle: f64, point: Vec2, mirrored: bool) -> f64 {
    let angle = (point.y - center.y).atan2(point.x - center.x) - start_angle;
    let degrees = angle.sin().atan2(angle.cos()).to_degrees();
    if mirrored { -degrees } else { degrees }
}

fn mirrored(matrix: &Matrix) -> bool {
    matches!(matrix.mirror, Some(Mirror::X | Mirror::Y))
}

/// An origin drop snaps first to the dragged column's base (the ergogen-style reset), then to part
/// geometry, then to the grid; Alt keeps the raw point.
pub fn snap_world_origin(
    point: Vec2,
    matrix: &Matrix,
    base: Vec2,
    input: &RailInput<'_>,
) -> (Vec2, Option<Vec2>) {
    if input.alt {
        return (point, None);
    }
    if (point.x - base.x).hypot(point.y - base.y) <= input.tolerance {
        return (base, Some(base));
    }
    if input.geometry_snap
        && let Some((at, _)) = snap_origin(point, input.origins, input.tolerance)
    {
        return (at, Some(at));
    }
    match grid_step(matrix.pitch, input.snap_fraction) {
        Some(step) => (
            Vec2 {
                x: (point.x / step.x).round() * step.x,
                y: (point.y / step.y).round() * step.y,
            },
            None,
        ),
        None => (point, None),
    }
}

fn stored_angle(matrix: &Matrix, column: u32, affect: &MatrixSplayAffect) -> f64 {
    matrix
        .column_layout
        .get(column as usize)
        .map_or(0.0, |record| {
            column_angle(record.splay, record.rotation, affect)
        })
}

fn set_matrix(matrix: Matrix) -> EditOperation {
    EditOperation::SetMatrix {
        matrix,
        definitions: None,
    }
}

impl RailGesture {
    /// The matrix or part this gesture edits.
    pub fn target_id(&self) -> &str {
        match self {
            Self::Placement { matrix, .. }
            | Self::Splay { matrix, .. }
            | Self::Origin { matrix, .. }
            | Self::MatrixMove { matrix, .. }
            | Self::MatrixRotate { matrix, .. } => &matrix.id,
            Self::PartValue { part_id, .. } | Self::PartRotate { part_id, .. } => part_id,
        }
    }

    /// The preview a pointer sample proposes, or `None` when it proposes nothing.
    pub fn sample(&self, input: &RailInput<'_>) -> Option<RailPreview> {
        let preview = |operation, value: String, delta| {
            Some(RailPreview {
                operation,
                value,
                delta,
                guide: None,
            })
        };
        match self {
            Self::Placement {
                matrix,
                parameter,
                basis,
            } => {
                let mut snap = if input.alt {
                    None
                } else {
                    grid_step(matrix.pitch, input.snap_fraction)
                };
                if matches!(parameter, PlacementParameter::Stagger { .. })
                    && basis.stagger_axis.x.abs() > basis.stagger_axis.y.abs()
                {
                    snap = snap.map(|step| Vec2 {
                        x: step.x,
                        y: step.x,
                    });
                }
                let start = placement_current(matrix, *parameter);
                let end = gesture_value(basis, *parameter, start, input.world_delta, snap);
                preview(
                    placement_operation(&matrix.id, *parameter, end),
                    format_placement_value(end),
                    placement_delta(start, end),
                )
            }
            Self::Splay {
                matrix,
                column,
                origin,
                start_angle,
                swing,
                affect,
                column_base,
            } => {
                if *swing {
                    let (world, guide) =
                        snap_world_origin(input.point, matrix, *column_base, input);
                    let delta = FeedbackDelta::Mm2(world.x - origin.x, world.y - origin.y);
                    let operation = EditOperation::SetMatrixSplay {
                        matrix_id: matrix.id.clone(),
                        column: *column,
                        change: MatrixSplayChange::OriginSwing { world: Some(world) },
                    };
                    return Some(RailPreview {
                        operation,
                        value: format!("X {:.2}, Y {:.2} mm", world.x, world.y),
                        delta,
                        guide,
                    });
                }
                let start = stored_angle(matrix, *column, affect);
                let turned = sweep(*origin, *start_angle, input.point, mirrored(matrix));
                let angle = if input.alt {
                    start + turned
                } else {
                    (start + turned).round()
                };
                let operation = EditOperation::SetMatrixSplay {
                    matrix_id: matrix.id.clone(),
                    column: *column,
                    change: MatrixSplayChange::Angle {
                        angle,
                        affect: affect.clone(),
                    },
                };
                Some(RailPreview {
                    operation,
                    value: format!("{angle:.1}°"),
                    delta: FeedbackDelta::Deg(angle - start),
                    guide: None,
                })
            }
            Self::Origin {
                matrix,
                column,
                swing,
                column_base,
                start_origin,
            } => {
                let (world, guide) = snap_world_origin(input.point, matrix, *column_base, input);
                let change = if *swing {
                    MatrixSplayChange::OriginSwing { world: Some(world) }
                } else {
                    MatrixSplayChange::Origin { world: Some(world) }
                };
                let operation = EditOperation::SetMatrixSplay {
                    matrix_id: matrix.id.clone(),
                    column: *column,
                    change,
                };
                Some(RailPreview {
                    operation,
                    value: format!("X {:.2}, Y {:.2} mm", world.x, world.y),
                    delta: FeedbackDelta::Mm2(world.x - start_origin.x, world.y - start_origin.y),
                    guide,
                })
            }
            Self::MatrixMove { matrix, anchor } => {
                let origin = solve_matrix_origin(matrix, *anchor, input.world_delta);
                // A drag outside every Follow direction changes nothing.
                (origin != matrix.origin).then(|| {
                    let mut next = matrix.clone();
                    next.origin = origin;
                    RailPreview {
                        operation: set_matrix(next),
                        value: format!("X {:.2}, Y {:.2} mm", origin.x, origin.y),
                        delta: FeedbackDelta::Mm2(
                            origin.x - matrix.origin.x,
                            origin.y - matrix.origin.y,
                        ),
                        guide: None,
                    }
                })
            }
            Self::MatrixRotate {
                matrix,
                anchor,
                center,
                start_angle,
            } => {
                let rotation = solve_matrix_rotation(
                    matrix.rotation.unwrap_or(0.0),
                    anchor.is_none_or(|a| a.follow.rotation),
                    sweep(*center, *start_angle, input.point, mirrored(matrix)),
                )?;
                let rotation = if input.alt {
                    rotation
                } else {
                    rotation.round()
                };
                let mut next = matrix.clone();
                next.rotation = Some(rotation);
                Some(RailPreview {
                    operation: set_matrix(next),
                    value: format!("{rotation:.1}°"),
                    delta: FeedbackDelta::Deg(rotation - matrix.rotation.unwrap_or(0.0)),
                    guide: None,
                })
            }
            Self::PartValue {
                part_id,
                axis,
                anchor,
                base_rotation,
                center,
                start_angle,
            } => {
                let (value, delta, change) = match axis {
                    PartAxis::Rotation => {
                        let rotation = round_half(
                            anchor.rotation + sweep(*center, *start_angle, input.point, false),
                            input.alt,
                        );
                        (
                            format!("{rotation:.1}°"),
                            FeedbackDelta::Deg(rotation - anchor.rotation),
                            PartAnchorValueChange::Rotation(rotation),
                        )
                    }
                    axis => {
                        let basis = part_placement_basis(*base_rotation, anchor.splay);
                        let mut snap = if input.alt {
                            None
                        } else {
                            grid_step(input.pitch, input.snap_fraction)
                        };
                        if matches!(axis, PartAxis::Stagger)
                            && basis.stagger_axis.x.abs() > basis.stagger_axis.y.abs()
                        {
                            snap = snap.map(|step| Vec2 {
                                x: step.x,
                                y: step.x,
                            });
                        }
                        let (parameter, start) = match axis {
                            PartAxis::Spread => (
                                PlacementParameter::ColumnSpread {
                                    column: anchor.column,
                                },
                                PlacementValue::Spread(anchor.spread),
                            ),
                            PartAxis::Stagger => (
                                PlacementParameter::Stagger {
                                    column: anchor.column,
                                },
                                PlacementValue::Stagger(anchor.stagger),
                            ),
                            _ => (
                                PlacementParameter::ColumnOffset {
                                    column: anchor.column,
                                },
                                PlacementValue::Offset(anchor.offset),
                            ),
                        };
                        let end = gesture_value(&basis, parameter, start, input.world_delta, snap);
                        let change = match end {
                            PlacementValue::Spread(v) => PartAnchorValueChange::Spread(v),
                            PlacementValue::Stagger(v) => PartAnchorValueChange::Stagger(v),
                            PlacementValue::Offset(v) => PartAnchorValueChange::Offset(v),
                            _ => return None,
                        };
                        (
                            format_placement_value(end),
                            placement_delta(start, end),
                            change,
                        )
                    }
                };
                Some(RailPreview {
                    operation: EditOperation::SetPartAnchorValues {
                        part_id: part_id.clone(),
                        change,
                    },
                    value,
                    delta,
                    guide: None,
                })
            }
            Self::PartRotate {
                part_id,
                center,
                start_angle,
                start_rotation,
            } => {
                let rotation = round_half(
                    start_rotation + sweep(*center, *start_angle, input.point, false),
                    input.alt,
                );
                Some(RailPreview {
                    operation: EditOperation::SetPartRotation {
                        part_id: part_id.clone(),
                        rotation,
                    },
                    value: format!("{rotation:.1}°"),
                    delta: FeedbackDelta::Deg(rotation - start_rotation),
                    guide: None,
                })
            }
        }
    }
}

/// A part rotation sweep snaps to half degrees; Alt keeps the raw angle.
fn round_half(value: f64, alt: bool) -> f64 {
    if alt {
        value
    } else {
        (value / 0.5).round() * 0.5
    }
}

/// What an arrow key on a focused handle moves.
#[derive(Clone, Debug, PartialEq)]
pub enum RailNudge {
    Placement(PlacementParameter),
    /// One arrow step of the splay origin or angle (only the horizontal pair turns the angle).
    SplayOrigin {
        column: u32,
    },
    SplayAngle {
        column: u32,
        affect: MatrixSplayAffect,
    },
    /// One arrow step of the matrix Origin or rotation.
    MatrixMove {
        anchor: Option<MatrixDragAnchor>,
    },
    MatrixRotate,
}

impl RailNudge {
    /// The edit one arrow press makes against `matrix`, or `None` when the press is not consumed.
    pub fn operation(
        &self,
        matrix: &Matrix,
        scene: &boardstudio_model::model::MatrixScene,
        key: ArrowKey,
        big: bool,
    ) -> Option<EditOperation> {
        let step = if big { 1.0 } else { 0.1 };
        let delta = match key {
            ArrowKey::Left => Vec2 { x: -step, y: 0.0 },
            ArrowKey::Right => Vec2 { x: step, y: 0.0 },
            ArrowKey::Up => Vec2 { x: 0.0, y: step },
            ArrowKey::Down => Vec2 { x: 0.0, y: -step },
        };
        let degrees = delta.x.signum() * if big { 5.0 } else { 1.0 };
        match self {
            Self::Placement(parameter) => {
                if let PlacementParameter::RowBend { .. } | PlacementParameter::KeyBend { .. } =
                    parameter
                {
                    let PlacementValue::Bend(current) = placement_current(matrix, *parameter)
                    else {
                        return None;
                    };
                    return (delta.x != 0.0).then(|| {
                        placement_operation(
                            &matrix.id,
                            *parameter,
                            PlacementValue::Bend(current + degrees),
                        )
                    });
                }
                let basis = SceneBasis::for_parameter(scene, *parameter)?;
                nudge_value(
                    &basis,
                    *parameter,
                    placement_current(matrix, *parameter),
                    key,
                    big,
                )
                .map(|value| placement_operation(&matrix.id, *parameter, value))
            }
            Self::SplayOrigin { column } => {
                let basis = scene.columns.iter().find(|basis| basis.column == *column)?;
                let world = Vec2 {
                    x: basis.splay_origin.x + delta.x,
                    y: basis.splay_origin.y + delta.y,
                };
                Some(EditOperation::SetMatrixSplay {
                    matrix_id: matrix.id.clone(),
                    column: *column,
                    change: MatrixSplayChange::Origin { world: Some(world) },
                })
            }
            Self::SplayAngle { column, affect } => {
                if delta.x == 0.0 {
                    return None;
                }
                let basis = scene.columns.iter().find(|basis| basis.column == *column)?;
                let angle = column_angle(basis.splay_angle, basis.rotation, affect) + degrees;
                Some(EditOperation::SetMatrixSplay {
                    matrix_id: matrix.id.clone(),
                    column: *column,
                    change: MatrixSplayChange::Angle {
                        angle,
                        affect: affect.clone(),
                    },
                })
            }
            Self::MatrixMove { anchor } => {
                let origin = solve_matrix_origin(matrix, *anchor, delta);
                (origin != matrix.origin).then(|| {
                    let mut next = matrix.clone();
                    next.origin = origin;
                    set_matrix(next)
                })
            }
            Self::MatrixRotate => {
                if delta.x == 0.0 {
                    return None;
                }
                let rotation = solve_matrix_rotation(
                    matrix.rotation.unwrap_or(0.0),
                    matrix.anchor.as_ref().is_none_or(|a| a.follow.rotation),
                    degrees,
                )?;
                let mut next = matrix.clone();
                next.rotation = Some(rotation);
                Some(set_matrix(next))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix() -> Matrix {
        serde_json::from_value(serde_json::json!({
            "id": "m", "rows": 2, "columns": 2, "pitch": {"x": 19.0, "y": 19.0}, "origin": {"x": 0.0, "y": 0.0},
            "definitionId": "switch", "partIds": [], "boardId": "b", "rotation": 0.0
        }))
        .unwrap()
    }

    fn input(delta: Vec2) -> RailInput<'static> {
        RailInput {
            point: Vec2 { x: 0.0, y: 0.0 },
            world_delta: delta,
            alt: false,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            snap_fraction: 0.0,
            geometry_snap: false,
            origins: &[],
            tolerance: 1.0,
        }
    }

    #[test]
    fn grid_step_is_a_pitch_fraction_or_absolute_millimetres() {
        let pitch = Vec2 { x: 19.0, y: 18.0 };
        assert_eq!(grid_step(pitch, 0.0), None);
        assert_eq!(grid_step(pitch, 0.5), Some(Vec2 { x: 9.5, y: 9.0 }));
        assert_eq!(grid_step(pitch, -0.25), Some(Vec2 { x: 0.25, y: 0.25 }));
    }

    #[test]
    fn detached_rotation_sweeps_in_half_degrees_and_alt_keeps_the_raw_angle() {
        let gesture = RailGesture::PartRotate {
            part_id: "p".into(),
            center: Vec2 { x: 0.0, y: 0.0 },
            start_angle: 0.0,
            start_rotation: 10.0,
        };
        let mut sample = input(Vec2 { x: 0.0, y: 0.0 });
        sample.point = Vec2 { x: 1.0, y: 0.1 };
        let snapped = gesture.sample(&sample).unwrap();
        assert_eq!(
            snapped.operation,
            EditOperation::SetPartRotation {
                part_id: "p".into(),
                rotation: 15.5
            }
        );
        sample.alt = true;
        let EditOperation::SetPartRotation { rotation, .. } =
            gesture.sample(&sample).unwrap().operation
        else {
            unreachable!()
        };
        assert!((rotation - (10.0 + 0.1_f64.atan2(1.0).to_degrees())).abs() < 1e-9);
    }

    #[test]
    fn origin_drops_snap_to_the_column_base_before_geometry_and_alt_bypasses() {
        let matrix = matrix();
        let mut sample = input(Vec2 { x: 0.0, y: 0.0 });
        sample.point = Vec2 { x: 10.2, y: 5.0 };
        let base = Vec2 { x: 10.0, y: 5.0 };
        assert_eq!(
            snap_world_origin(sample.point, &matrix, base, &sample),
            (base, Some(base))
        );
        sample.alt = true;
        assert_eq!(
            snap_world_origin(sample.point, &matrix, base, &sample),
            (Vec2 { x: 10.2, y: 5.0 }, None)
        );
    }

    #[test]
    fn a_move_outside_every_follow_direction_proposes_nothing() {
        let matrix = matrix();
        let gesture = RailGesture::MatrixMove {
            matrix: matrix.clone(),
            anchor: None,
        };
        assert!(gesture.sample(&input(Vec2 { x: 0.0, y: 0.0 })).is_none());
        let moved = gesture.sample(&input(Vec2 { x: 2.0, y: -1.0 })).unwrap();
        assert_eq!(moved.delta, FeedbackDelta::Mm2(2.0, -1.0));
    }

    #[test]
    fn rail_nudge_covers_bend_splay_rotate_anchored_move_and_mirror_y() {
        use boardstudio_model::matrix::kernel::ComposedFrame;
        use boardstudio_model::model::{
            Follow, MatrixAnchor, MatrixColumnBasis, MatrixScene, MatrixSceneCell, Pose2,
        };
        let reflected_frame = ComposedFrame {
            origin: Vec2::default(),
            rotation: 0.0,
            axis_x: Vec2 { x: -1.0, y: 0.0 },
            axis_y: Vec2 { x: 0.0, y: 1.0 },
        };
        let scene = MatrixScene {
            matrix_id: "m".into(),
            cells: vec![MatrixSceneCell {
                row: 0,
                column: 0,
                enabled: true,
                member_id: None,
                pose: Pose2 {
                    at: Vec2::default(),
                    rotation: 0.0,
                },
                shift_frame: reflected_frame,
                offset_frame: reflected_frame,
                row_shift: Vec2::default(),
                key_shift: Vec2::default(),
                row_bend: 0.0,
                key_bend: 0.0,
                cumulative_shift: Vec2::default(),
                cumulative_bend: 0.0,
            }],
            columns: vec![MatrixColumnBasis {
                column: 0,
                base: Vec2 { x: 0.0, y: 0.0 },
                rotation: 5.0,
                relative_origin: Vec2 { x: 0.0, y: 0.0 },
                spread_axis: Some(Vec2 { x: 1.0, y: 0.0 }),
                splay_origin: Vec2 { x: 0.0, y: 0.0 },
                splay_angle: 10.0,
                custom_origin: false,
                axis_x: Vec2 { x: -1.0, y: 0.0 },
                axis_y: Vec2 { x: 0.0, y: 1.0 },
                stagger_axis: Vec2 { x: 0.0, y: 1.0 },
            }],
            companions: Vec::new(),
        };
        let mut m = matrix();
        // 1) Bend degrees (Up ignored, Right +1, Shift+Left -5).
        let bend = RailNudge::Placement(PlacementParameter::RowBend { row: 0 });
        assert!(bend.operation(&m, &scene, ArrowKey::Up, false).is_none());
        assert_eq!(
            bend.operation(&m, &scene, ArrowKey::Right, false),
            Some(placement_operation(
                "m",
                PlacementParameter::RowBend { row: 0 },
                PlacementValue::Bend(1.0)
            ))
        );
        assert_eq!(
            bend.operation(&m, &scene, ArrowKey::Left, true),
            Some(placement_operation(
                "m",
                PlacementParameter::RowBend { row: 0 },
                PlacementValue::Bend(-5.0)
            ))
        );
        // 2) Splay angle step (horizontal only).
        let splay = RailNudge::SplayAngle {
            column: 0,
            affect: MatrixSplayAffect::Following,
        };
        assert!(splay.operation(&m, &scene, ArrowKey::Up, false).is_none());
        assert_eq!(
            splay.operation(&m, &scene, ArrowKey::Right, false),
            Some(EditOperation::SetMatrixSplay {
                matrix_id: "m".into(),
                column: 0,
                change: MatrixSplayChange::Angle {
                    angle: 11.0,
                    affect: MatrixSplayAffect::Following,
                },
            })
        );
        // 3) Mirror-Y reflected X axis on ColumnOffset nudge: Right gives -0.1 along stored X.
        assert_eq!(
            RailNudge::Placement(PlacementParameter::ColumnOffset { column: 0 }).operation(
                &m,
                &scene,
                ArrowKey::Right,
                false
            ),
            Some(placement_operation(
                "m",
                PlacementParameter::ColumnOffset { column: 0 },
                PlacementValue::Offset(Vec2 { x: -0.1, y: 0.0 })
            ))
        );
        // 4) Rotate with follow.rotation == false returns None; true rotates.
        m.anchor = Some(MatrixAnchor {
            matrix_id: "other".into(),
            row: 0,
            column: 0,
            follow: Follow {
                x: true,
                y: true,
                rotation: false,
            },
            aggregate: None,
        });
        assert!(
            RailNudge::MatrixRotate
                .operation(&m, &scene, ArrowKey::Right, false)
                .is_none()
        );
        m.anchor.as_mut().unwrap().follow.rotation = true;
        assert!(
            RailNudge::MatrixRotate
                .operation(&m, &scene, ArrowKey::Right, false)
                .is_some()
        );
        // 5) Anchored move inverse-rotates through anchor pose (90 deg, Shift+Up -> origin.x += 1).
        let anchored = RailNudge::MatrixMove {
            anchor: Some(MatrixDragAnchor {
                key_pose: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 90.0,
                },
                follow: Follow::default(),
                transport: crate::matrix_transform_operation::MatrixDragTransport::World,
            }),
        };
        let Some(EditOperation::SetMatrix { matrix: moved, .. }) =
            anchored.operation(&m, &scene, ArrowKey::Up, true)
        else {
            panic!("expected SetMatrix from anchored move")
        };
        assert!((moved.origin.x - 1.0).abs() < 1e-9 && moved.origin.y.abs() < 1e-9);
    }
}
