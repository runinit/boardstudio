use super::{
    BoardPartChoice, OffsetDrafts, default_offset_axes, offset_axes_from_value, offset_axes_value,
    offset_drafts, offset_values,
};
use crate::presentation::inspector::LayoutConstraintValues;
use boardstudio_core::model::{
    Constraint, ConstraintEndpoint, ConstraintRelation, MirrorAxis, OffsetAxes, Vec2,
};

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

#[wasm_bindgen_test::wasm_bindgen_test]
fn a_matrix_key_source_defaults_to_part_axes_and_any_other_source_to_board_axes() {
    assert_eq!(default_offset_axes(true), OffsetAxes::Part);
    assert_eq!(default_offset_axes(false), OffsetAxes::Board);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn the_axes_select_value_round_trips_for_both_axes() {
    for axes in [OffsetAxes::Board, OffsetAxes::Part] {
        assert_eq!(offset_axes_from_value(offset_axes_value(axes)), axes);
    }
    assert_eq!(offset_axes_value(OffsetAxes::Board), "board");
    assert_eq!(offset_axes_value(OffsetAxes::Part), "part");
}

#[wasm_bindgen_test::wasm_bindgen_test]
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

#[wasm_bindgen_test::wasm_bindgen_test]
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

#[wasm_bindgen_test::wasm_bindgen_test]
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

#[wasm_bindgen_test::wasm_bindgen_test]
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

#[wasm_bindgen_test::wasm_bindgen_test]
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

#[wasm_bindgen_test::wasm_bindgen_test]
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

#[wasm_bindgen_test::wasm_bindgen_test]
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
    assert!(super::position_input_disabled(true));
    assert!(!super::position_input_disabled(false));
    assert_eq!(super::position_commit_value("12.5", true), None);
    assert_eq!(super::position_commit_value("12.5", false), Some(12.5));
}
