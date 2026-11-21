//! Placement parameter values, deltas and their feedback text. Plain logic: compiled
//! natively for tests.
use crate::matrix_transform_operation::{PlacementParameter, PlacementValue};
use boardstudio_model::model::{EditOperation, Matrix, MatrixPlacementChange, Vec2};
use boardstudio_web_ui_model::transform_feedback::FeedbackDelta;

/// Ticket 06: the committed value text for a placement result, matching the
/// token value style ("Spread 28.57 mm" comes from the name plus this).
pub(super) fn format_placement_value(value: PlacementValue) -> String {
    match value {
        PlacementValue::Stagger(value) | PlacementValue::Spread(value) => {
            format!("{value:.2} mm")
        }
        PlacementValue::Offset(value) | PlacementValue::Shift(value) => {
            format!("X {:.2}, Y {:.2} mm", value.x, value.y)
        }
        PlacementValue::Bend(value) => format!("{value:.1}°"),
        PlacementValue::Padding(value) => format!("{value:.2} mm"),
    }
}

/// Ticket 06: a placement drag delta in the toast's display units.
pub(super) fn placement_feedback_delta(delta: PlacementDelta) -> FeedbackDelta {
    match delta {
        PlacementDelta::Stagger(value)
        | PlacementDelta::Spread(value)
        | PlacementDelta::Padding(value) => FeedbackDelta::Mm(value),
        PlacementDelta::Offset(value) | PlacementDelta::Shift(value) => {
            FeedbackDelta::Mm2(value.x, value.y)
        }
        PlacementDelta::Bend(value) => FeedbackDelta::Deg(value),
    }
}

pub(super) fn stagger_parameter(column: u32, quick_offset: bool) -> PlacementParameter {
    if quick_offset {
        PlacementParameter::ColumnOffset { column }
    } else {
        PlacementParameter::Stagger { column }
    }
}

/// The parameter's value in `matrix`; an index past the saved vector reads as zero,
/// and an absent Spread or Padding reads the pitch so a drag that starts from a
/// default writes an explicit step.
pub(super) fn placement_current(matrix: &Matrix, parameter: PlacementParameter) -> PlacementValue {
    match parameter {
        PlacementParameter::Stagger { column } => PlacementValue::Stagger(
            matrix
                .column_layout
                .get(column as usize)
                .map(|record| &record.stagger)
                .copied()
                .unwrap_or(0.0),
        ),
        PlacementParameter::ColumnOffset { column } => PlacementValue::Offset(
            matrix
                .column_layout
                .get(column as usize)
                .map(|record| &record.offset)
                .copied()
                .unwrap_or_default(),
        ),
        PlacementParameter::RowOffset { row } => PlacementValue::Offset(
            matrix
                .row_layout
                .get(row as usize)
                .map(|record| &record.offset)
                .copied()
                .unwrap_or_default(),
        ),
        PlacementParameter::RowShift { row } => PlacementValue::Shift(
            matrix
                .row_layout
                .get(row as usize)
                .map(|record| &record.shift)
                .copied()
                .unwrap_or_default(),
        ),
        PlacementParameter::KeyShift { row, column } => PlacementValue::Shift(
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
                .and_then(|cell| cell.shift)
                .unwrap_or_default(),
        ),
        PlacementParameter::RowBend { row } => PlacementValue::Bend(
            matrix
                .row_layout
                .get(row as usize)
                .map(|record| &record.bend)
                .copied()
                .unwrap_or(0.0),
        ),
        PlacementParameter::KeyBend { row, column } => PlacementValue::Bend(
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
                .and_then(|cell| cell.bend)
                .unwrap_or(0.0),
        ),
        PlacementParameter::ColumnSpread { column } => PlacementValue::Spread(
            matrix
                .column_layout
                .get(column as usize)
                .and_then(|record| record.spread)
                .unwrap_or(matrix.pitch.x),
        ),
        PlacementParameter::RowPadding { row } => PlacementValue::Padding(
            matrix
                .row_layout
                .get(row as usize)
                .and_then(|record| record.padding)
                .unwrap_or(matrix.pitch.y),
        ),
    }
}

/// The parameter and absolute value a placement operation sets. Absent Spread
/// and Padding resolve against the matrix pitch, matching `placement_current`,
/// so a drag that lands on a default still compares and deltas exactly.
pub(super) fn placement_target(
    operation: &EditOperation,
    matrix: &Matrix,
) -> Option<(PlacementParameter, PlacementValue)> {
    let EditOperation::SetMatrixPlacement {
        matrix_id: target,
        change,
    } = operation
    else {
        return None;
    };
    if target != &matrix.id {
        return None;
    }
    match change {
        MatrixPlacementChange::ColumnStagger { column, value } => Some((
            PlacementParameter::Stagger { column: *column },
            PlacementValue::Stagger(*value),
        )),
        MatrixPlacementChange::ColumnOffset { column, value } => Some((
            PlacementParameter::ColumnOffset { column: *column },
            PlacementValue::Offset(*value),
        )),
        MatrixPlacementChange::RowOffset { row, value } => Some((
            PlacementParameter::RowOffset { row: *row },
            PlacementValue::Offset(*value),
        )),
        MatrixPlacementChange::RowShift { row, value } => Some((
            PlacementParameter::RowShift { row: *row },
            PlacementValue::Shift(*value),
        )),
        MatrixPlacementChange::KeyShift { row, column, value } => Some((
            PlacementParameter::KeyShift {
                row: *row,
                column: *column,
            },
            PlacementValue::Shift(*value),
        )),
        MatrixPlacementChange::RowBend { row, value } => Some((
            PlacementParameter::RowBend { row: *row },
            PlacementValue::Bend(*value),
        )),
        MatrixPlacementChange::KeyBend { row, column, value } => Some((
            PlacementParameter::KeyBend {
                row: *row,
                column: *column,
            },
            PlacementValue::Bend(*value),
        )),
        MatrixPlacementChange::ColumnSpread { column, value } => Some((
            PlacementParameter::ColumnSpread { column: *column },
            PlacementValue::Spread(value.unwrap_or(matrix.pitch.x)),
        )),
        MatrixPlacementChange::RowPadding { row, value } => Some((
            PlacementParameter::RowPadding { row: *row },
            PlacementValue::Padding(value.unwrap_or(matrix.pitch.y)),
        )),
        _ => None,
    }
}

/// The net change of a placement parameter over a drag, from start to snapped value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PlacementDelta {
    Stagger(f64),
    Offset(Vec2),
    Shift(Vec2),
    Bend(f64),
    Spread(f64),
    Padding(f64),
}

pub(super) fn placement_delta(
    start: PlacementValue,
    end: PlacementValue,
) -> Option<PlacementDelta> {
    match (start, end) {
        (PlacementValue::Stagger(start), PlacementValue::Stagger(end)) => {
            Some(PlacementDelta::Stagger(end - start))
        }
        (PlacementValue::Offset(start), PlacementValue::Offset(end)) => {
            Some(PlacementDelta::Offset(Vec2 {
                x: end.x - start.x,
                y: end.y - start.y,
            }))
        }
        (PlacementValue::Shift(start), PlacementValue::Shift(end)) => {
            Some(PlacementDelta::Shift(Vec2 {
                x: end.x - start.x,
                y: end.y - start.y,
            }))
        }
        (PlacementValue::Bend(start), PlacementValue::Bend(end)) => {
            Some(PlacementDelta::Bend(end - start))
        }
        (PlacementValue::Spread(start), PlacementValue::Spread(end)) => {
            Some(PlacementDelta::Spread(end - start))
        }
        (PlacementValue::Padding(start), PlacementValue::Padding(end)) => {
            Some(PlacementDelta::Padding(end - start))
        }
        _ => None,
    }
}

pub(super) fn apply_placement_delta(
    current: PlacementValue,
    delta: PlacementDelta,
) -> Option<PlacementValue> {
    match (current, delta) {
        (PlacementValue::Stagger(value), PlacementDelta::Stagger(delta)) => {
            Some(PlacementValue::Stagger(value + delta))
        }
        (PlacementValue::Offset(value), PlacementDelta::Offset(delta)) => {
            Some(PlacementValue::Offset(Vec2 {
                x: value.x + delta.x,
                y: value.y + delta.y,
            }))
        }
        (PlacementValue::Shift(value), PlacementDelta::Shift(delta)) => {
            Some(PlacementValue::Shift(Vec2 {
                x: value.x + delta.x,
                y: value.y + delta.y,
            }))
        }
        (PlacementValue::Bend(value), PlacementDelta::Bend(delta)) => {
            Some(PlacementValue::Bend(value + delta))
        }
        (PlacementValue::Spread(value), PlacementDelta::Spread(delta)) => {
            Some(PlacementValue::Spread(value + delta))
        }
        (PlacementValue::Padding(value), PlacementDelta::Padding(delta)) => {
            Some(PlacementValue::Padding(value + delta))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::{ColumnLayout, MatrixCell, RowLayout};

    fn vec2(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    fn matrix() -> Matrix {
        Matrix {
            id: "matrix".into(),
            name: None,
            rows: 2,
            columns: 2,
            pitch: vec2(19.05, 18.0),
            origin: vec2(0.0, 0.0),
            definition_id: "switch".into(),
            part_ids: vec![],
            board_id: None,
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            column_layout: vec![ColumnLayout {
                stagger: 2.5,
                offset: vec2(1.0, -1.0),
                spread: Some(20.0),
                ..ColumnLayout::default()
            }],
            row_layout: vec![RowLayout {
                shift: vec2(0.5, 0.25),
                bend: 3.0,
                offset: vec2(-2.0, 4.0),
                padding: Some(17.0),
            }],
            anchor: None,
            cells: vec![MatrixCell {
                shift: Some(vec2(0.1, 0.2)),
                bend: Some(-4.0),
                deleted: false,
                row: 1,
                column: 1,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            }],
        }
    }

    #[test]
    fn current_values_read_saved_records_zero_past_the_vector_and_pitch_for_defaults() {
        use PlacementParameter as P;
        use PlacementValue as V;
        let matrix = matrix();
        for (parameter, expected) in [
            (P::Stagger { column: 0 }, V::Stagger(2.5)),
            (P::Stagger { column: 1 }, V::Stagger(0.0)),
            (P::ColumnOffset { column: 0 }, V::Offset(vec2(1.0, -1.0))),
            (P::ColumnOffset { column: 1 }, V::Offset(Vec2::default())),
            (P::RowOffset { row: 0 }, V::Offset(vec2(-2.0, 4.0))),
            (P::RowShift { row: 0 }, V::Shift(vec2(0.5, 0.25))),
            (P::RowShift { row: 1 }, V::Shift(Vec2::default())),
            (P::KeyShift { row: 1, column: 1 }, V::Shift(vec2(0.1, 0.2))),
            (P::KeyShift { row: 0, column: 0 }, V::Shift(Vec2::default())),
            (P::RowBend { row: 0 }, V::Bend(3.0)),
            (P::KeyBend { row: 1, column: 1 }, V::Bend(-4.0)),
            (P::KeyBend { row: 0, column: 1 }, V::Bend(0.0)),
            (P::ColumnSpread { column: 0 }, V::Spread(20.0)),
            (P::ColumnSpread { column: 1 }, V::Spread(19.05)),
            (P::RowPadding { row: 0 }, V::Padding(17.0)),
            (P::RowPadding { row: 1 }, V::Padding(18.0)),
        ] {
            assert_eq!(
                placement_current(&matrix, parameter),
                expected,
                "{parameter:?}"
            );
        }
    }

    #[test]
    fn a_placement_operation_names_its_target_and_defaults_resolve_against_the_pitch() {
        let matrix = matrix();
        let operation = |change| EditOperation::SetMatrixPlacement {
            matrix_id: "matrix".into(),
            change,
        };
        assert_eq!(
            placement_target(
                &operation(MatrixPlacementChange::ColumnSpread {
                    column: 1,
                    value: None
                }),
                &matrix
            ),
            Some((
                PlacementParameter::ColumnSpread { column: 1 },
                PlacementValue::Spread(19.05)
            ))
        );
        assert_eq!(
            placement_target(
                &operation(MatrixPlacementChange::RowPadding {
                    row: 0,
                    value: None
                }),
                &matrix
            ),
            Some((
                PlacementParameter::RowPadding { row: 0 },
                PlacementValue::Padding(18.0)
            ))
        );
        assert_eq!(
            placement_target(
                &operation(MatrixPlacementChange::KeyBend {
                    row: 1,
                    column: 0,
                    value: 2.0
                }),
                &matrix
            ),
            Some((
                PlacementParameter::KeyBend { row: 1, column: 0 },
                PlacementValue::Bend(2.0)
            ))
        );
        let other = EditOperation::SetMatrixPlacement {
            matrix_id: "other".into(),
            change: MatrixPlacementChange::ColumnStagger {
                column: 0,
                value: 1.0,
            },
        };
        assert_eq!(placement_target(&other, &matrix), None, "another matrix");
    }

    #[test]
    fn deltas_reapply_to_a_moved_value_and_refuse_mismatched_kinds() {
        use PlacementValue as V;
        for (start, end, later, expected) in [
            (
                V::Stagger(1.0),
                V::Stagger(3.5),
                V::Stagger(10.0),
                V::Stagger(12.5),
            ),
            (
                V::Spread(19.0),
                V::Spread(18.0),
                V::Spread(20.0),
                V::Spread(19.0),
            ),
            (
                V::Padding(18.0),
                V::Padding(18.5),
                V::Padding(1.0),
                V::Padding(1.5),
            ),
            (V::Bend(2.0), V::Bend(-1.0), V::Bend(5.0), V::Bend(2.0)),
            (
                V::Offset(vec2(1.0, 1.0)),
                V::Offset(vec2(3.0, 0.0)),
                V::Offset(vec2(0.0, 0.0)),
                V::Offset(vec2(2.0, -1.0)),
            ),
            (
                V::Shift(vec2(0.0, 0.0)),
                V::Shift(vec2(0.5, 0.5)),
                V::Shift(vec2(1.0, 1.0)),
                V::Shift(vec2(1.5, 1.5)),
            ),
        ] {
            let delta = placement_delta(start, end).expect("same kind");
            assert_eq!(
                apply_placement_delta(later, delta),
                Some(expected),
                "{delta:?}"
            );
        }
        assert_eq!(placement_delta(V::Stagger(1.0), V::Spread(1.0)), None);
        assert_eq!(
            apply_placement_delta(V::Bend(1.0), PlacementDelta::Stagger(1.0)),
            None
        );
    }

    #[test]
    fn feedback_text_matches_the_token_value_style() {
        use PlacementValue as V;
        for (value, text) in [
            (V::Stagger(2.0), "2.00 mm"),
            (V::Spread(28.571), "28.57 mm"),
            (V::Padding(18.0), "18.00 mm"),
            (V::Offset(vec2(1.0, -2.5)), "X 1.00, Y -2.50 mm"),
            (V::Shift(vec2(0.126, 0.0)), "X 0.13, Y 0.00 mm"),
            (V::Bend(-3.26), "-3.3°"),
        ] {
            assert_eq!(format_placement_value(value), text);
        }
        for (delta, expected) in [
            (PlacementDelta::Stagger(1.0), FeedbackDelta::Mm(1.0)),
            (PlacementDelta::Spread(-1.0), FeedbackDelta::Mm(-1.0)),
            (PlacementDelta::Padding(0.5), FeedbackDelta::Mm(0.5)),
            (
                PlacementDelta::Offset(vec2(1.0, 2.0)),
                FeedbackDelta::Mm2(1.0, 2.0),
            ),
            (
                PlacementDelta::Shift(vec2(-1.0, 0.0)),
                FeedbackDelta::Mm2(-1.0, 0.0),
            ),
            (PlacementDelta::Bend(4.0), FeedbackDelta::Deg(4.0)),
        ] {
            assert_eq!(placement_feedback_delta(delta), expected);
        }
        assert_eq!(
            stagger_parameter(3, true),
            PlacementParameter::ColumnOffset { column: 3 }
        );
        assert_eq!(
            stagger_parameter(3, false),
            PlacementParameter::Stagger { column: 3 }
        );
    }
}
