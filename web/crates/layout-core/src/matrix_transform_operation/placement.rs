//! Placement parameters and values: the one place a web intent becomes a
//! `SetMatrixPlacement`, plus the drag-gesture and keyboard-nudge value solvers.

use boardstudio_model::model::{
    EditOperation, MatrixColumnBasis, MatrixPlacementChange, MatrixScene, Vec2,
};

/// The matrix parameter a placement edit names. Core owns the rules for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementParameter {
    /// Relative Y step of a column, inherited by every later column.
    Stagger { column: u32 },
    /// Independent X/Y adjustment of one column, in that column's frame.
    ColumnOffset { column: u32 },
    /// Independent X/Y adjustment of one row, in column 0's frame.
    RowOffset { row: u32 },
    /// Carried X/Y move of one row, in that row's running frame; later rows
    /// inherit it through Core's running frame.
    RowShift { row: u32 },
    /// Carried X/Y move of one key, in that key's running frame.
    KeyShift { row: u32, column: u32 },
    /// Carried turn of one row, in degrees; later rows inherit it.
    RowBend { row: u32 },
    /// Carried turn of one key, in degrees; later rows in its column inherit it.
    KeyBend { row: u32, column: u32 },
    /// Inherited X step of one column; absent means the matrix pitch.
    ColumnSpread { column: u32 },
    /// Y step of one row; absent means the matrix pitch.
    RowPadding { row: u32 },
}

/// An absolute parameter value in matrix coordinates. Spread and Padding are
/// resolved: the pitch fills an absent stored step, so a drag that starts from
/// a default writes an explicit step and never a stored copy of the pitch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlacementValue {
    Stagger(f64),
    Offset(Vec2),
    Shift(Vec2),
    Bend(f64),
    Spread(f64),
    Padding(f64),
}

/// The arrow keys a nudge can consume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowKey {
    Left,
    Right,
    Up,
    Down,
}

/// The frame a parameter is edited in: the column's stagger axis, its offset
/// axes, and the running-frame axes a carried Shift uses. A row Shift uses
/// column 0's running frame; a row offset uses column 0's offset frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneBasis {
    pub stagger_axis: Vec2,
    pub axis_x: Vec2,
    pub axis_y: Vec2,
    pub shift_axis_x: Vec2,
    pub shift_axis_y: Vec2,
    pub spread_axis: Option<Vec2>,
}

impl SceneBasis {
    pub fn from_column(basis: &MatrixColumnBasis) -> Self {
        Self {
            stagger_axis: basis.stagger_axis,
            axis_x: basis.axis_x,
            axis_y: basis.axis_y,
            shift_axis_x: basis.axis_x,
            shift_axis_y: basis.axis_y,
            spread_axis: basis.spread_axis,
        }
    }

    /// The basis for a parameter, or `None` when the scene has no such row or
    /// column. Row parameters read the column-0 key's frames; a key Shift reads
    /// its own key's running frame. Padding has no drag gesture, so its basis
    /// only feeds the exhaustiveness of the lookup.
    pub fn for_parameter(scene: &MatrixScene, parameter: PlacementParameter) -> Option<Self> {
        let cell_coords = match parameter {
            PlacementParameter::ColumnOffset { column } => Some((0, column)),
            PlacementParameter::RowOffset { row }
            | PlacementParameter::RowShift { row }
            | PlacementParameter::RowBend { row } => Some((row, 0)),
            PlacementParameter::KeyShift { row, column }
            | PlacementParameter::KeyBend { row, column } => Some((row, column)),
            PlacementParameter::Stagger { .. }
            | PlacementParameter::ColumnSpread { .. }
            | PlacementParameter::RowPadding { .. } => None,
        };
        if let Some((row, column)) = cell_coords {
            let cell = crate::scene_cell_in(scene, row, column)?;
            return Some(Self {
                stagger_axis: cell.shift_frame.axis_y,
                axis_x: cell.offset_frame.axis_x,
                axis_y: cell.offset_frame.axis_y,
                shift_axis_x: cell.shift_frame.axis_x,
                shift_axis_y: cell.shift_frame.axis_y,
                spread_axis: None,
            });
        }
        let column = match parameter {
            PlacementParameter::Stagger { column }
            | PlacementParameter::ColumnSpread { column } => column,
            _ => 0,
        };
        scene
            .columns
            .iter()
            .find(|basis| basis.column == column)
            .map(Self::from_column)
    }
}

/// Ticket 05: the Bend drag rate and snap, matching the prototype's bend
/// handle (half a degree per millimetre of horizontal travel, snapped to
/// half degrees unless Alt is held).
pub const BEND_DRAG_DEGREES_PER_MM: f64 = 0.5;
pub const BEND_DRAG_SNAP_DEGREES: f64 = 0.5;

/// The only place a web intent becomes a `SetMatrixPlacement`.
pub fn placement_operation(
    matrix_id: &str,
    parameter: PlacementParameter,
    value: PlacementValue,
) -> EditOperation {
    let change = match (parameter, value) {
        (PlacementParameter::Stagger { column }, PlacementValue::Stagger(value)) => {
            MatrixPlacementChange::ColumnStagger { column, value }
        }
        (PlacementParameter::ColumnOffset { column }, PlacementValue::Offset(value)) => {
            MatrixPlacementChange::ColumnOffset { column, value }
        }
        (PlacementParameter::RowOffset { row }, PlacementValue::Offset(value)) => {
            MatrixPlacementChange::RowOffset { row, value }
        }
        (PlacementParameter::RowShift { row }, PlacementValue::Shift(value)) => {
            MatrixPlacementChange::RowShift { row, value }
        }
        (PlacementParameter::KeyShift { row, column }, PlacementValue::Shift(value)) => {
            MatrixPlacementChange::KeyShift { row, column, value }
        }
        (PlacementParameter::RowBend { row }, PlacementValue::Bend(value)) => {
            MatrixPlacementChange::RowBend { row, value }
        }
        (PlacementParameter::KeyBend { row, column }, PlacementValue::Bend(value)) => {
            MatrixPlacementChange::KeyBend { row, column, value }
        }
        (PlacementParameter::ColumnSpread { column }, PlacementValue::Spread(value)) => {
            MatrixPlacementChange::ColumnSpread {
                column,
                value: Some(value),
            }
        }
        (PlacementParameter::RowPadding { row }, PlacementValue::Padding(value)) => {
            MatrixPlacementChange::RowPadding {
                row,
                value: Some(value),
            }
        }
        (parameter, value) => panic!("{value:?} is not a value of {parameter:?}"),
    };
    EditOperation::SetMatrixPlacement {
        matrix_id: matrix_id.to_owned(),
        change,
    }
}

/// The one-shot conversion of a column's offset Y into stagger. Core refuses it when the
/// geometry would move; the Inspector asks `convert_offset_available` first.
pub fn convert_offset_operation(matrix_id: &str, column: u32) -> EditOperation {
    EditOperation::SetMatrixPlacement {
        matrix_id: matrix_id.to_owned(),
        change: MatrixPlacementChange::ColumnOffsetToStagger { column },
    }
}

/// An Inspector field value: the typed number replaces one component of the current value.
pub fn placement_value(
    parameter: PlacementParameter,
    current: PlacementValue,
    set: PlacementSet,
) -> PlacementValue {
    match (parameter, current, set) {
        (PlacementParameter::Stagger { .. }, _, PlacementSet::Stagger(value)) => {
            PlacementValue::Stagger(value)
        }
        (_, PlacementValue::Offset(offset), PlacementSet::OffsetX(x)) => {
            PlacementValue::Offset(Vec2 { x, y: offset.y })
        }
        (_, PlacementValue::Offset(offset), PlacementSet::OffsetY(y)) => {
            PlacementValue::Offset(Vec2 { x: offset.x, y })
        }
        (_, PlacementValue::Shift(shift), PlacementSet::ShiftX(x)) => {
            PlacementValue::Shift(Vec2 { x, y: shift.y })
        }
        (_, PlacementValue::Shift(shift), PlacementSet::ShiftY(y)) => {
            PlacementValue::Shift(Vec2 { x: shift.x, y })
        }
        (_, _, PlacementSet::Bend(value)) => PlacementValue::Bend(value),
        (_, _, PlacementSet::Spread(value)) => PlacementValue::Spread(value),
        (_, _, PlacementSet::Padding(value)) => PlacementValue::Padding(value),
        (_, _, set) => panic!("{set:?} does not fit {parameter:?}"),
    }
}

/// What an Inspector field edit sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlacementSet {
    Stagger(f64),
    OffsetX(f64),
    OffsetY(f64),
    ShiftX(f64),
    ShiftY(f64),
    Bend(f64),
    Spread(f64),
    Padding(f64),
}

/// A drag sample: the world delta since pointer-down, projected onto the scene axes and
/// added to the start value, then snapped to the grid step (`None` skips snapping, as Alt
/// does). Returns the absolute value, so it equals the Inspector's value for the same place.
/// A carried Shift projects onto the running frame Core applies it in, not the offset
/// frame an independent offset uses.
pub fn gesture_value(
    basis: &SceneBasis,
    parameter: PlacementParameter,
    start: PlacementValue,
    world_delta: Vec2,
    snap: Option<Vec2>,
) -> PlacementValue {
    match (parameter, start) {
        (PlacementParameter::Stagger { .. }, PlacementValue::Stagger(start)) => {
            let axis = basis.stagger_axis;
            let along = dot(world_delta, axis) / dot(axis, axis);
            PlacementValue::Stagger(snapped(start + along, snap.map(|step| step.y)))
        }
        (_, PlacementValue::Offset(start)) => {
            let local = solve(basis.axis_x, basis.axis_y, world_delta);
            PlacementValue::Offset(Vec2 {
                x: snapped(start.x + local.x, snap.map(|step| step.x)),
                y: snapped(start.y + local.y, snap.map(|step| step.y)),
            })
        }
        (_, PlacementValue::Shift(start)) => {
            let local = solve(basis.shift_axis_x, basis.shift_axis_y, world_delta);
            PlacementValue::Shift(Vec2 {
                x: snapped(start.x + local.x, snap.map(|step| step.x)),
                y: snapped(start.y + local.y, snap.map(|step| step.y)),
            })
        }
        (_, PlacementValue::Spread(start)) => {
            let axis = basis.spread_axis.unwrap_or(basis.axis_x);
            let along = dot(world_delta, axis) / dot(axis, axis);
            PlacementValue::Spread(snapped(start + along, snap.map(|step| step.x)))
        }
        // A Bend drag turns horizontal world travel into degrees at the
        // prototype bend handle's rate: half a degree per millimetre, snapped
        // to half degrees (Alt keeps the raw angle). The grid step never
        // reaches a bend: it measures millimetres, not the running frame.
        // Padding steps along the pre-row frame no handle exposes, so it
        // stays Inspector-only.
        (_, PlacementValue::Bend(start)) => PlacementValue::Bend(snapped(
            start - world_delta.x * BEND_DRAG_DEGREES_PER_MM,
            snap.map(|_| BEND_DRAG_SNAP_DEGREES),
        )),
        (_, PlacementValue::Padding(_)) => {
            panic!("row Padding has no drag gesture; use the Inspector field")
        }
        (parameter, start) => panic!("{start:?} is not a value of {parameter:?}"),
    }
}

/// A keyboard step (0.1 mm, Shift 1 mm), exact and not grid-snapped. Stagger consumes the
/// arrow pair closest to the stagger axis on screen and ignores the perpendicular pair;
/// offsets step in screen directions converted through the frame.
pub fn nudge_value(
    basis: &SceneBasis,
    parameter: PlacementParameter,
    current: PlacementValue,
    key: ArrowKey,
    big: bool,
) -> Option<PlacementValue> {
    let step = if big { 1.0 } else { 0.1 };
    let delta = match key {
        ArrowKey::Left => Vec2 { x: -step, y: 0.0 },
        ArrowKey::Right => Vec2 { x: step, y: 0.0 },
        ArrowKey::Up => Vec2 { x: 0.0, y: step },
        ArrowKey::Down => Vec2 { x: 0.0, y: -step },
    };
    if matches!(parameter, PlacementParameter::Stagger { .. }) {
        let axis = basis.stagger_axis;
        let vertical = axis.y.abs() >= axis.x.abs();
        if vertical != matches!(key, ArrowKey::Up | ArrowKey::Down) {
            return None;
        }
    }
    if matches!(parameter, PlacementParameter::ColumnSpread { .. }) {
        // A Spread nudge consumes only the arrow pair along the step: a
        // perpendicular press is not consumed, so it never submits a no-op.
        let axis = basis.spread_axis.unwrap_or(basis.axis_x);
        let horizontal = axis.x.abs() >= axis.y.abs();
        if horizontal != matches!(key, ArrowKey::Left | ArrowKey::Right) {
            return None;
        }
    }
    Some(gesture_value(basis, parameter, current, delta, None))
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

/// Coordinates of `delta` in the frame `(x_axis, y_axis)`: `delta = a * x_axis + b * y_axis`.
fn solve(x_axis: Vec2, y_axis: Vec2, delta: Vec2) -> Vec2 {
    let determinant = x_axis.x * y_axis.y - x_axis.y * y_axis.x;
    if determinant.abs() < f64::EPSILON {
        return Vec2 { x: 0.0, y: 0.0 };
    }
    Vec2 {
        x: (delta.x * y_axis.y - delta.y * y_axis.x) / determinant,
        y: (x_axis.x * delta.y - x_axis.y * delta.x) / determinant,
    }
}

/// Rounds to the nearest multiple of `step`; no step, or a non-positive one, leaves the value.
fn snapped(value: f64, step: Option<f64>) -> f64 {
    match step {
        Some(step) if step > 0.0 => (value / step).round() * step,
        _ => value,
    }
}
