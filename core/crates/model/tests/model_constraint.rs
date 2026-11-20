use boardstudio_model::model::*;

fn held(axes: Option<OffsetAxes>, x: f64, rotation: f64) -> Constraint {
    let (id, source_part_id, target_part_id) = ("c".to_owned(), "a".to_owned(), "b".to_owned());
    match axes {
        Some(axes) => Constraint::offset(
            axes,
            id,
            source_part_id,
            target_part_id,
            Vec2 { x, y: -4.0 },
            rotation,
        ),
        None => Constraint::Mirror {
            id,
            source_part_id,
            target_part_id,
            axis: MirrorAxis::Vertical,
            coordinate: x,
        },
    }
}

#[test]
fn offset_builds_the_kind_its_axes_name_and_carries_every_field() {
    let offset = Vec2 { x: 3.0, y: -4.0 };
    let build = |axes| Constraint::offset(axes, "c".into(), "a".into(), "b".into(), offset, 5.0);
    assert_eq!(
        build(OffsetAxes::Board),
        Constraint::Offset {
            id: "c".into(),
            source_part_id: "a".into(),
            target_part_id: "b".into(),
            offset,
            rotation: 5.0,
        }
    );
    assert_eq!(
        build(OffsetAxes::Part),
        Constraint::PartOffset {
            id: "c".into(),
            source_part_id: "a".into(),
            target_part_id: "b".into(),
            offset,
            rotation: 5.0,
        }
    );
}

#[test]
fn offset_axes_and_numbers_read_both_offset_kinds_and_nothing_for_a_mirror() {
    let numbers = Some((Vec2 { x: 1.0, y: -4.0 }, 5.0));
    let board = held(Some(OffsetAxes::Board), 1.0, 5.0);
    let part = held(Some(OffsetAxes::Part), 1.0, 5.0);
    let mirror = held(None, 1.0, 5.0);
    assert_eq!(board.offset_axes(), Some(OffsetAxes::Board));
    assert_eq!(part.offset_axes(), Some(OffsetAxes::Part));
    assert_eq!(mirror.offset_axes(), None);
    assert_eq!(board.offset_numbers(), numbers);
    assert_eq!(part.offset_numbers(), numbers);
    assert_eq!(mirror.offset_numbers(), None);
    assert!(!board.is_mirror() && !part.is_mirror() && mirror.is_mirror());
}

#[test]
fn a_non_finite_number_in_any_kind_is_not_finite() {
    for axes in [Some(OffsetAxes::Board), Some(OffsetAxes::Part), None] {
        assert!(held(axes, 1.0, 5.0).is_finite(), "{axes:?} finite");
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(!held(axes, bad, 5.0).is_finite(), "{axes:?} x {bad}");
        }
    }
    for axes in [OffsetAxes::Board, OffsetAxes::Part] {
        assert!(
            !held(Some(axes), 1.0, f64::NAN).is_finite(),
            "{axes:?} rotation"
        );
    }
}
