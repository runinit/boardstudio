//! Pure Layout component inspector draft logic: the offset source, axes and number
//! drafts a part's constraint form shows, and the position-commit rules. Compiled natively
//! so the rules are tested under `cargo test`.

use boardstudio_model::model::{Constraint, ConstraintRelation, MirrorAxis, OffsetAxes, Vec2};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardPartChoice {
    pub id: String,
    pub reference: String,
    /// A matrix key: a new offset from it defaults to its own axes.
    pub matrix_key: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutConstraintValues {
    Offset { offset: Vec2, rotation: f64 },
    PartOffset { offset: Vec2, rotation: f64 },
    Mirror { axis: MirrorAxis, coordinate: f64 },
    Framed { relation: ConstraintRelation },
}

/// A new offset from a matrix key follows the key's own axes; any other source uses the board's.
pub fn default_offset_axes(source_is_matrix_key: bool) -> OffsetAxes {
    if source_is_matrix_key {
        OffsetAxes::Part
    } else {
        OffsetAxes::Board
    }
}

/// The default axes for an offset from the board part `source_id`.
pub fn default_axes_for_source(board_parts: &[BoardPartChoice], source_id: &str) -> OffsetAxes {
    default_offset_axes(
        board_parts
            .iter()
            .any(|part| part.id == source_id && part.matrix_key),
    )
}

/// The value the Offset axes `<select>` uses for `axes`, and the inverse.
pub fn offset_axes_value(axes: OffsetAxes) -> &'static str {
    match axes {
        OffsetAxes::Board => "board",
        OffsetAxes::Part => "part",
    }
}

pub fn offset_axes_from_value(value: &str) -> OffsetAxes {
    if value == offset_axes_value(OffsetAxes::Part) {
        OffsetAxes::Part
    } else {
        OffsetAxes::Board
    }
}

/// What the Properties form shows for one part's layout constraint: the source part, the
/// offset axes and the Offset X / Y and rotation drafts. The form's first render, every
/// accepted-constraint resync and the reset all take their values from here.
#[derive(Clone, Debug, PartialEq)]
pub struct OffsetDrafts {
    pub source_part_id: String,
    pub axes: OffsetAxes,
    pub offset_x: String,
    pub offset_y: String,
    pub rotation: String,
}

/// The drafts for `part_id` given the constraint that drives it, if any. A saved Framed
/// constraint keeps its endpoint source, even when that part is on the paired board. A legacy
/// constraint with an off-board source falls back to the first other part. Saved offset axes
/// and numbers are preserved; otherwise axes follow the source and numbers are zero.
pub fn offset_drafts(
    constraint: Option<&Constraint>,
    board_parts: &[BoardPartChoice],
    part_id: &str,
) -> OffsetDrafts {
    let source_part_id = constraint
        .filter(|constraint| {
            matches!(constraint, Constraint::Framed { .. })
                || board_parts
                    .iter()
                    .any(|part| part.id == constraint.source())
        })
        .map(Constraint::source)
        .map(str::to_owned)
        .or_else(|| {
            board_parts
                .iter()
                .find(|part| part.id != part_id)
                .map(|part| part.id.clone())
        })
        .unwrap_or_default();
    let axes = constraint
        .and_then(Constraint::offset_axes)
        .unwrap_or_else(|| default_axes_for_source(board_parts, &source_part_id));
    let (offset, rotation) = constraint
        .and_then(Constraint::offset_numbers)
        .unwrap_or((Vec2::default(), 0.0));
    OffsetDrafts {
        source_part_id,
        axes,
        offset_x: offset.x.to_string(),
        offset_y: offset.y.to_string(),
        rotation: rotation.to_string(),
    }
}

/// The constraint values the Save button submits for the chosen axes.
pub fn offset_values(axes: OffsetAxes, offset: Vec2, rotation: f64) -> LayoutConstraintValues {
    match axes {
        OffsetAxes::Board => LayoutConstraintValues::Offset { offset, rotation },
        OffsetAxes::Part => LayoutConstraintValues::PartOffset { offset, rotation },
    }
}

pub fn position_input_disabled(locked: bool) -> bool {
    locked
}

pub fn position_commit_value(draft: &str, locked: bool) -> Option<f64> {
    if locked {
        return None;
    }
    draft.parse::<f64>().ok().filter(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::ConstraintEndpoint;

    fn offsets() -> (Constraint, Constraint, Constraint) {
        let board = Constraint::Offset {
            id: "c".into(),
            source_part_id: "a".into(),
            target_part_id: "b".into(),
            offset: Vec2 { x: 1.0, y: -4.0 },
            rotation: 5.0,
        };
        let part = Constraint::PartOffset {
            id: "c".into(),
            source_part_id: "a".into(),
            target_part_id: "b".into(),
            offset: Vec2 { x: 1.0, y: -4.0 },
            rotation: 5.0,
        };
        let mirror = Constraint::Mirror {
            id: "m".into(),
            source_part_id: "a".into(),
            target_part_id: "b".into(),
            axis: MirrorAxis::Vertical,
            coordinate: 0.0,
        };
        (board, part, mirror)
    }

    #[test]
    fn a_matrix_key_source_defaults_to_part_axes_and_any_other_source_to_board_axes() {
        assert_eq!(default_offset_axes(true), OffsetAxes::Part);
        assert_eq!(default_offset_axes(false), OffsetAxes::Board);
    }

    #[test]
    fn the_axes_select_value_round_trips_for_both_axes() {
        for axes in [OffsetAxes::Board, OffsetAxes::Part] {
            assert_eq!(offset_axes_from_value(offset_axes_value(axes)), axes);
        }
        assert_eq!(offset_axes_value(OffsetAxes::Board), "board");
        assert_eq!(offset_axes_value(OffsetAxes::Part), "part");
    }

    #[test]
    fn the_axes_choice_and_the_numbers_read_both_offset_kinds() {
        let (board, part, mirror) = offsets();
        assert_eq!(board.offset_axes(), Some(OffsetAxes::Board));
        assert_eq!(part.offset_axes(), Some(OffsetAxes::Part));
        assert_eq!(mirror.offset_axes(), None);
        let numbers = (Vec2 { x: 1.0, y: -4.0 }, 5.0);
        assert_eq!(board.offset_numbers(), Some(numbers));
        assert_eq!(part.offset_numbers(), Some(numbers));
        assert_eq!(mirror.offset_numbers(), None);
    }

    /// A key, a diode and a plain part, in board order.
    fn board() -> Vec<BoardPartChoice> {
        let part = |id: &str, matrix_key| BoardPartChoice {
            id: id.into(),
            reference: id.to_uppercase(),
            matrix_key,
        };
        vec![
            part("key", true),
            part("diode", false),
            part("plain", false),
        ]
    }

    fn zero_drafts(source: &str, axes: OffsetAxes) -> OffsetDrafts {
        OffsetDrafts {
            source_part_id: source.into(),
            axes,
            offset_x: "0".into(),
            offset_y: "0".into(),
            rotation: "0".into(),
        }
    }

    #[test]
    fn a_saved_offset_shows_its_own_source_axes_and_numbers_for_either_kind() {
        let (board_axes, part_axes, _) = offsets();
        let saved = |axes| OffsetDrafts {
            source_part_id: "a".into(),
            axes,
            offset_x: "1".into(),
            offset_y: "-4".into(),
            rotation: "5".into(),
        };
        let parts = [
            BoardPartChoice {
                id: "a".into(),
                reference: "A".into(),
                matrix_key: true,
            },
            BoardPartChoice {
                id: "b".into(),
                reference: "B".into(),
                matrix_key: false,
            },
        ];
        // The source is a matrix key, whose default is part axes: the saved kind still wins.
        assert_eq!(
            offset_drafts(Some(&board_axes), &parts, "b"),
            saved(OffsetAxes::Board)
        );
        assert_eq!(
            offset_drafts(Some(&part_axes), &parts, "b"),
            saved(OffsetAxes::Part)
        );
    }

    #[test]
    fn an_unconstrained_part_defaults_the_axes_by_its_first_other_part() {
        // The diode's first other part is the key; the key's is the diode. The default is
        // taken for the part now selected, not for the one the form first opened on.
        assert_eq!(
            offset_drafts(None, &board(), "diode"),
            zero_drafts("key", OffsetAxes::Part)
        );
        assert_eq!(
            offset_drafts(None, &board(), "key"),
            zero_drafts("diode", OffsetAxes::Board)
        );
        assert_eq!(
            offset_drafts(None, &board(), "plain"),
            zero_drafts("key", OffsetAxes::Part)
        );
    }

    #[test]
    fn a_mirror_constraint_or_an_off_board_source_shows_the_default_and_zeros() {
        let (_, _, mirror) = offsets();
        assert_eq!(
            offset_drafts(Some(&mirror), &board(), "diode"),
            zero_drafts("key", OffsetAxes::Part)
        );
        // `a` is not on this board, so the form falls back to the first other part.
        let (off_board, _, _) = offsets();
        let drafts = offset_drafts(Some(&off_board), &board(), "key");
        assert_eq!(drafts.source_part_id, "diode");
        assert_eq!(
            drafts,
            OffsetDrafts {
                source_part_id: "diode".into(),
                axes: OffsetAxes::Board,
                offset_x: "1".into(),
                offset_y: "-4".into(),
                rotation: "5".into(),
            }
        );
    }

    #[test]
    fn an_off_board_part_offset_keeps_saved_axes_instead_of_the_fallback_default() {
        let (_, off_board, _) = offsets();
        assert_eq!(
            offset_drafts(Some(&off_board), &board(), "key"),
            OffsetDrafts {
                source_part_id: "diode".into(),
                axes: OffsetAxes::Part,
                offset_x: "1".into(),
                offset_y: "-4".into(),
                rotation: "5".into(),
            }
        );
    }

    #[test]
    fn an_off_board_framed_source_keeps_its_endpoint_axes_and_numbers() {
        let framed = Constraint::Framed {
            id: "framed-offset".into(),
            source: ConstraintEndpoint::Part {
                part_id: "paired-board-source".into(),
            },
            target: ConstraintEndpoint::Part {
                part_id: "key".into(),
            },
            relation: ConstraintRelation::PartOffset {
                offset: Vec2 { x: 1.0, y: -4.0 },
                rotation: 5.0,
            },
        };
        assert_eq!(
            offset_drafts(Some(&framed), &board(), "key"),
            OffsetDrafts {
                source_part_id: "paired-board-source".into(),
                axes: OffsetAxes::Part,
                offset_x: "1".into(),
                offset_y: "-4".into(),
                rotation: "5".into(),
            }
        );
    }

    #[test]
    fn saving_builds_the_constraint_kind_the_axes_choice_names() {
        let offset = Vec2 { x: 2.0, y: 3.0 };
        assert_eq!(
            offset_values(OffsetAxes::Board, offset, 7.0),
            LayoutConstraintValues::Offset {
                offset,
                rotation: 7.0
            }
        );
        assert_eq!(
            offset_values(OffsetAxes::Part, offset, 7.0),
            LayoutConstraintValues::PartOffset {
                offset,
                rotation: 7.0
            }
        );
        assert!(position_input_disabled(true));
        assert!(!position_input_disabled(false));
        assert_eq!(position_commit_value("12.5", true), None);
        assert_eq!(position_commit_value("12.5", false), Some(12.5));
    }
}
