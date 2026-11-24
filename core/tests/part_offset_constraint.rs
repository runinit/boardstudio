//! A `part-offset` constraint holds its target at a fixed distance and angle along its
//! source part's own axes, so a companion held to a key keeps its pose in the key's frame
//! whenever the key turns. An `offset` constraint keeps its board-axes meaning.
use crate::support::fixture::{self, scene_and_document as scene};
use boardstudio_core::{CommittedScene, CoreEngine, EditOutcome, model::*};

const KEY: &str = "matrix/m/r0c1";
const HELD: &str = "matrix/m/r0c1/held";

fn edit(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    scene(engine.commit_edit(fixture::commit("edit", "edit", doc.revision, operation)))
}

fn matrix() -> Matrix {
    let cell = |column: u32| MatrixCell {
        assemblies: vec![MatrixAssembly {
            id: "held".into(),
            definition_id: "diode".into(),
            offset: Vec2 { x: 1.0, y: -4.0 },
            rotation: None,
            side: None,
        }],
        ..fixture::cell(0, column)
    };
    Matrix {
        column_layout: vec![
            boardstudio_core::model::ColumnLayout::default(),
            boardstudio_core::model::ColumnLayout::default(),
            boardstudio_core::model::ColumnLayout::default(),
        ],
        cells: vec![cell(0), cell(1), cell(2)],
        ..fixture::matrix("m", 1, 3)
    }
}

/// A three-key matrix whose second key holds a diode through `constraint`.
fn fixture(constraint: impl FnOnce() -> Constraint) -> (CoreEngine, ProjectDoc) {
    let mut doc = ProjectDoc::empty("part-offset", "Part offset");
    doc.definitions = serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]},
        {"id":"diode","name":"Diode","kind":"passive","courtyard":[],"pads":[]},
    ]))
    .unwrap();
    let mut engine = CoreEngine::new();
    let (_, doc) = scene(engine.open(doc));
    let (_, mut doc) = edit(&mut engine, &doc, fixture::set_matrix(matrix()));
    doc.constraints.push(constraint());
    let mut engine = CoreEngine::new();
    scene(engine.open(doc));
    let (_, doc) = scene(engine.snapshot());
    (engine, doc)
}

fn held_by_part_axes() -> Constraint {
    held_by(
        OffsetAxes::Part,
        "held-key",
        KEY,
        HELD,
        Vec2 { x: 1.0, y: -4.0 },
        0.0,
    )
}

fn assert_key_frame_pose_kept(before: &ProjectDoc, after: &ProjectDoc) {
    let (was_at, was_turn) = fixture::relative_pose(before, KEY, HELD);
    let (now_at, now_turn) = fixture::relative_pose(after, KEY, HELD);
    assert!(
        (was_at.x - now_at.x).abs() < 1e-9
            && (was_at.y - now_at.y).abs() < 1e-9
            && (was_turn - now_turn).abs() < 1e-9,
        "key-relative {was_at:?} turn {was_turn} -> {now_at:?} turn {now_turn}"
    );
}

/// A key motion with its postcondition: the key must observably move, then the
/// held companion must keep its pose in the key's frame.
type PoseCheck = Box<dyn FnOnce(&ProjectDoc, &ProjectDoc)>;

fn key_motion(label: &str, before: &ProjectDoc) -> (EditOperation, PoseCheck) {
    match label {
        "column-splay" | "following-splay" => (
            EditOperation::SetMatrixSplay {
                matrix_id: "m".into(),
                column: 1,
                change: MatrixSplayChange::Angle {
                    angle: 30.0,
                    affect: if label == "column-splay" {
                        MatrixSplayAffect::Column
                    } else {
                        MatrixSplayAffect::Following
                    },
                },
            },
            Box::new(|before: &ProjectDoc, after: &ProjectDoc| {
                let turned = fixture::part_by_id(after, KEY).pose.rotation
                    - fixture::part_by_id(before, KEY).pose.rotation;
                assert!((turned - 30.0).abs() < 1e-9, "key turned {turned}");
            }),
        ),
        "matrix-rotation" => {
            let mut turned = before.matrices[0].clone();
            turned.rotation = Some(25.0);
            (
                fixture::set_matrix(turned),
                Box::new(|before: &ProjectDoc, after: &ProjectDoc| {
                    assert!(
                        (fixture::part_by_id(after, KEY).pose.rotation
                            - fixture::part_by_id(before, KEY).pose.rotation)
                            .abs()
                            > 1e-6,
                        "the matrix rotation must turn the key"
                    );
                }),
            )
        }
        _ => {
            let key_at = fixture::part_by_id(before, KEY).pose.at;
            (
                EditOperation::MoveParts {
                    positions: vec![Position {
                        id: KEY.into(),
                        at: Vec2 {
                            x: key_at.x + 5.0,
                            y: key_at.y + 2.0,
                        },
                    }],
                },
                Box::new(move |_: &ProjectDoc, after: &ProjectDoc| {
                    assert!(
                        (fixture::part_by_id(after, KEY).pose.at.x - (key_at.x + 5.0)).abs() < 1e-9,
                        "the key did not move"
                    );
                }),
            )
        }
    }
}

#[test]
fn a_part_offset_companion_keeps_its_key_frame_pose_through_key_motion() {
    for label in [
        "column-splay",
        "following-splay",
        "matrix-rotation",
        "key-move",
    ] {
        let (mut engine, before) = fixture(held_by_part_axes);
        let (operation, check_moved) = key_motion(label, &before);
        let (_, after) = edit(&mut engine, &before, operation);
        check_moved(&before, &after);
        assert_key_frame_pose_kept(&before, &after);
    }
}

#[test]
fn dragging_a_companion_retunes_its_offset_on_its_own_axes() {
    // Drag the companion 3 mm along the board's X axis with the key turned 30 degrees.
    // A part-offset companion stores the drag rotated back into the key's frame:
    // (3 cos 30, -3 sin 30) on top of (1, -4). A board-axes companion stores the
    // drag as it is: (1, -4) + (3, 0), whatever the key's turn.
    for (label, setup, part, expected) in [
        (
            "part-offset",
            held_by_part_axes as fn() -> Constraint,
            true,
            (1.0 + 2.598_076_211_353_316, -4.0 - 1.5),
        ),
        (
            "board-axes",
            held_by_board_axes as fn() -> Constraint,
            false,
            (4.0, -4.0),
        ),
    ] {
        let (mut engine, before) = fixture(setup);
        let (_, splayed) = edit(
            &mut engine,
            &before,
            EditOperation::SetMatrixSplay {
                matrix_id: "m".into(),
                column: 1,
                change: MatrixSplayChange::Angle {
                    angle: 30.0,
                    affect: MatrixSplayAffect::Column,
                },
            },
        );
        let turn = fixture::part_by_id(&splayed, KEY).pose.rotation;
        assert!(
            (turn - 30.0).abs() < 1e-9,
            "{label}: the key must be turned, found {turn}"
        );
        let start = fixture::part_by_id(&splayed, HELD).pose.at;
        let target = Vec2 {
            x: start.x + 3.0,
            y: start.y,
        };
        let (_, after) = edit(
            &mut engine,
            &splayed,
            EditOperation::MoveParts {
                positions: vec![Position {
                    id: HELD.into(),
                    at: target,
                }],
            },
        );
        let landed = fixture::part_by_id(&after, HELD).pose.at;
        assert!(
            (landed.x - target.x).abs() < 1e-9 && (landed.y - target.y).abs() < 1e-9,
            "{label}: companion did not land on the drag: {landed:?}"
        );
        let (offset, rotation) = match after
            .constraints
            .iter()
            .find(|constraint| constraint.target() == HELD)
        {
            Some(Constraint::PartOffset {
                offset, rotation, ..
            }) if part => (*offset, *rotation),
            Some(Constraint::Offset {
                offset, rotation, ..
            }) if !part => (*offset, *rotation),
            other => panic!("{label}: the held constraint was lost: {other:?}"),
        };
        assert!(
            (offset.x - expected.0).abs() < 1e-9 && (offset.y - expected.1).abs() < 1e-9,
            "{label}: offset {offset:?} is not the drag on the companion's axes"
        );
        assert_eq!(rotation, 0.0, "{label}");
    }
}

fn held_by_board_axes() -> Constraint {
    held_by(
        OffsetAxes::Board,
        "held-key",
        KEY,
        HELD,
        Vec2 { x: 1.0, y: -4.0 },
        0.0,
    )
}

/// A key-held companion, after its column is splayed 30 degrees.
fn splayed(constraint: impl FnOnce() -> Constraint) -> (CoreEngine, ProjectDoc) {
    let (mut engine, before) = fixture(constraint);
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrixSplay {
            matrix_id: "m".into(),
            column: 1,
            change: MatrixSplayChange::Angle {
                angle: 30.0,
                affect: MatrixSplayAffect::Column,
            },
        },
    );
    (engine, after)
}

fn convert(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    constraint_id: &str,
    to: OffsetAxes,
) -> Result<CommittedScene, String> {
    engine.commit_edit(fixture::commit(
        "convert",
        "convert",
        doc.revision,
        EditOperation::ConvertOffsetConstraint {
            constraint_id: constraint_id.into(),
            to,
        },
    ))
}

fn assert_every_part_kept(before: &ProjectDoc, after: &ProjectDoc) {
    assert_eq!(before.parts.len(), after.parts.len());
    for (was, now) in before.parts.iter().zip(&after.parts) {
        assert_eq!(was.id, now.id);
        assert!(
            (was.pose.at.x - now.pose.at.x).abs() < 1e-9
                && (was.pose.at.y - now.pose.at.y).abs() < 1e-9
                && (was.pose.rotation - now.pose.rotation).abs() < 1e-9,
            "{} moved on conversion: {:?} -> {:?}",
            was.id,
            was.pose,
            now.pose
        );
    }
}

fn held_constraint(doc: &ProjectDoc) -> &Constraint {
    doc.constraints
        .iter()
        .find(|constraint| constraint.target() == HELD)
        .expect("the held companion keeps a constraint")
}

#[test]
fn converting_between_board_and_part_axes_moves_no_part_and_keeps_the_id() {
    // (1, -4) converts between the board axes and the key's frame at a 30 degree turn.
    for (label, setup, to, expected) in [
        (
            "board-to-part",
            held_by_board_axes as fn() -> Constraint,
            OffsetAxes::Part,
            (-1.1339745962155614, -3.9641016151377544),
        ),
        (
            "part-to-board",
            held_by_part_axes as fn() -> Constraint,
            OffsetAxes::Board,
            (2.866_025_403_784_439, -2.9641016151377544),
        ),
    ] {
        let (mut engine, before) = splayed(setup);
        let (_, after) = scene(convert(&mut engine, &before, "held-key", to));
        assert_every_part_kept(&before, &after);
        let (offset, rotation) = match held_constraint(&after) {
            Constraint::PartOffset {
                offset, rotation, ..
            } if to == OffsetAxes::Part => (*offset, *rotation),
            Constraint::Offset {
                offset, rotation, ..
            } if to == OffsetAxes::Board => (*offset, *rotation),
            other => panic!("{label}: the constraint did not convert: {other:?}"),
        };
        assert_eq!(held_constraint(&after).id(), "held-key", "{label}");
        assert!(
            (offset.x - expected.0).abs() < 1e-9
                && (offset.y - expected.1).abs() < 1e-9
                && rotation.abs() < 1e-9,
            "{label}: offset {offset:?} turn {rotation}"
        );
    }
}

#[test]
fn converting_an_unknown_constraint_is_refused_and_changes_nothing() {
    let (mut engine, before) = splayed(held_by_part_axes);
    let Err(message) = convert(
        &mut engine,
        &before,
        "no-such-constraint",
        OffsetAxes::Board,
    ) else {
        panic!("an unknown constraint must be refused");
    };
    assert!(message.contains("no-such-constraint"), "{message}");
    let (_, now) = scene(engine.snapshot());
    assert_eq!(now.constraints, before.constraints);
    assert_every_part_kept(&before, &now);
}

#[test]
fn converting_a_mirror_constraint_is_refused_and_changes_nothing() {
    for to in [OffsetAxes::Board, OffsetAxes::Part] {
        let (mut engine, before) = splayed(|| Constraint::Mirror {
            id: "held-key".into(),
            source_part_id: KEY.into(),
            target_part_id: HELD.into(),
            axis: MirrorAxis::Vertical,
            coordinate: 5.0,
        });
        let Err(message) = convert(&mut engine, &before, "held-key", to) else {
            panic!("a mirror has no offset axes to convert to {to:?}");
        };
        assert!(message.contains("mirror"), "{message}");
        let (_, now) = scene(engine.snapshot());
        assert_eq!(now.constraints, before.constraints);
        assert_eq!(now.revision, before.revision);
        assert_every_part_kept(&before, &now);
    }
}

#[test]
fn a_part_offset_constraint_has_its_own_kind_on_the_wire_and_round_trips() {
    let wire = serde_json::json!({
        "kind": "part-offset", "id": "held-key", "sourcePartId": KEY, "targetPartId": HELD,
        "offset": {"x": 1.0, "y": -4.0}, "rotation": 0.0
    });
    let constraint: Constraint = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(constraint, held_by_part_axes());
    assert_eq!(serde_json::to_value(&constraint).unwrap(), wire);
}

#[test]
fn offset_keeps_its_board_axes_wire_shape() {
    let encoded = serde_json::to_value(held_by_board_axes()).unwrap();
    assert_eq!(encoded["kind"], "offset");
    assert!(encoded.get("axes").is_none());
}

#[test]
fn an_unknown_constraint_kind_is_refused_as_an_older_build_would_refuse_part_offset() {
    let future = serde_json::json!({
        "kind": "some-future-kind", "id": "held-key", "sourcePartId": KEY,
        "targetPartId": HELD, "offset": {"x": 1.0, "y": -4.0}, "rotation": 0.0
    });
    assert!(serde_json::from_value::<Constraint>(future).is_err());
}

#[test]
fn the_part_offset_kind_is_persisted_under_the_current_format_version() {
    let (_, document) = splayed(held_by_board_axes);
    let encoded = serde_json::to_value(document).unwrap();
    assert_eq!(
        encoded["formatVersion"],
        boardstudio_core::document_format::CURRENT_VERSION
    );
}

#[test]
fn previewing_a_drag_of_a_constrained_companion_leaves_its_stored_offset_untouched() {
    for (kind, constraint) in [
        ("offset", held_by_board_axes as fn() -> Constraint),
        ("part-offset", held_by_part_axes as fn() -> Constraint),
    ] {
        let (mut engine, before) = splayed(constraint);
        let start = fixture::part_by_id(&before, HELD).pose.at;
        let _ = engine.apply_edit(fixture::edit(
            "preview",
            "drag",
            before.revision,
            EditPhase::Preview,
            EditOperation::MoveParts {
                positions: vec![Position {
                    id: HELD.into(),
                    at: Vec2 {
                        x: start.x + 3.0,
                        y: start.y,
                    },
                }],
            },
        ));
        let (_, now) = scene(engine.snapshot());
        assert_eq!(
            now.constraints, before.constraints,
            "{kind} was changed by a preview"
        );
        assert_eq!(
            fixture::part_by_id(&now, HELD).pose,
            fixture::part_by_id(&before, HELD).pose,
            "{kind} moved"
        );
    }
}

fn held_by(
    axes: OffsetAxes,
    id: &str,
    source: &str,
    target: &str,
    offset: Vec2,
    rotation: f64,
) -> Constraint {
    Constraint::offset(
        axes,
        id.to_owned(),
        source.to_owned(),
        target.to_owned(),
        offset,
        rotation,
    )
}

/// A drag moves a companion; it never turns it, so the stored turn keeps its exact bits.
#[test]
fn dragging_a_held_companion_keeps_the_exact_stored_rotation_of_either_kind() {
    for axes in [OffsetAxes::Board, OffsetAxes::Part] {
        for stored in [-0.0_f64, 7.5] {
            let (mut engine, before) = fixture(|| {
                held_by(
                    axes,
                    "held-key",
                    KEY,
                    HELD,
                    Vec2 { x: 1.0, y: -4.0 },
                    stored,
                )
            });
            let kept = |doc: &ProjectDoc| match held_constraint(doc) {
                Constraint::Offset { rotation, .. } | Constraint::PartOffset { rotation, .. } => {
                    rotation.to_bits()
                }
                Constraint::Mirror { .. } | Constraint::Framed { .. } => {
                    panic!("the constraint changed relation kind")
                }
            };
            assert_eq!(kept(&before), stored.to_bits(), "{axes:?} precondition");
            let start = fixture::part_by_id(&before, HELD).pose.at;
            let (_, after) = edit(
                &mut engine,
                &before,
                EditOperation::MoveParts {
                    positions: vec![Position {
                        id: HELD.into(),
                        at: Vec2 {
                            x: start.x + 3.0,
                            y: start.y - 2.0,
                        },
                    }],
                },
            );
            assert_eq!(
                kept(&after),
                stored.to_bits(),
                "{axes:?} drag rewrote the stored rotation {stored:?}"
            );
        }
    }
}

/// Dragging an authoritative part retunes a saved `Constraint::Framed` whose target binds to a
/// live linked-mirror endpoint while preserving the stored `-0.0` rotation bits.
#[test]
fn dragging_the_authority_retunes_a_saved_linked_target() {
    // The saved target is on the beta half, but alpha/T owns its placement.
    // This pair reflects x about 10 and preserves y.
    let document: ProjectDoc = serde_json::from_value(serde_json::json!({
        "format": "boardstudio/v2",
        "formatVersion": boardstudio_core::document_format::CURRENT_VERSION,
        "revision": 0,
        "id": "framed", "name": "Framed drag",
        "parameters": {}, "matrices": [], "nets": [], "outline": [],
        "boards": [
            {"id": "alpha", "name": "Alpha", "partIds": ["alpha/T"],
             "outlineIds": [], "netIds": [], "thickness": 1.6},
            {"id": "beta", "name": "Beta", "partIds": [],
             "outlineIds": [], "netIds": [], "thickness": 1.6}
        ],
        "caseBodies": [], "materials": [], "assets": [],
        "definitions": [{"id": "d", "name": "Fixture", "kind": "custom", "pads": [], "courtyard": []}],
        "parts": [
            {"id": "source", "definitionId": "d", "reference": "S1", "side": "front",
             "pose": {"at": {"x": 1, "y": 2}, "rotation": 0}},
            {"id": "alpha/T", "definitionId": "d", "reference": "T1", "side": "front",
             "pose": {"at": {"x": 16, "y": 4}, "rotation": 180}}
        ],
        "mirroredPairs": [{
            "id": "pair",
            "source": {"kind": "secondBoard", "id": "alpha"},
            "destination": {"kind": "secondBoard", "id": "beta"},
            "mapping": {"at": {"x": 20, "y": 0}, "rotation": 0},
            "overrides": []
        }],
        "constraints": [{
            "kind": "framed", "id": "drag",
            "source": {"kind": "part", "partId": "source"},
            "target": {"kind": "linked-mirror", "pairId": "pair", "sourcePartId": "alpha/T", "mirrorPartId": "4/pair/part/alpha/T"},
            "relation": {"kind": "offset", "offset": {"x": 3, "y": 2}, "rotation": -0.0}
        }],
        "hardware": {
            "topology": "split", "transport": "none", "boards": [],
            "instances": [
                {"id": "alpha-instance", "name": "Alpha", "boardId": "alpha", "half": "left", "role": "primary", "flipped": false},
                {"id": "beta-instance", "name": "Beta", "boardId": "beta", "half": "right", "role": "secondary", "flipped": true}
            ],
            "pairs": [{
                "id": "pair", "members": ["alpha-instance", "beta-instance"], "constructionSource": "alpha-instance",
                "mapping": {"at": {"x": 20, "y": 0}, "rotation": 0},
                "construction": {"method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "plateThickness": 1.5, "plateFoamThickness": 0, "bottomFoamThickness": 0,
                    "bottomThickness": 1.5, "plateToPcb": 5, "wallThickness": 2, "clearance": 0.2},
                "halves": {"sourceBoardId": "alpha", "mirrorBoardId": "beta"}
            }]
        }
    })).unwrap();
    let mut engine = CoreEngine::new();
    let (_, opened) = fixture::scene_and_document(engine.open(document));
    assert_eq!(
        fixture::part_by_id(&opened, "alpha/T").pose.at,
        Vec2 { x: 16.0, y: 4.0 }
    );
    let at = Vec2 { x: 13.0, y: 8.0 };
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "alpha/T".into(),
            at,
        }],
    };
    let Ok(EditOutcome::Preview(scene)) = engine.apply_edit(fixture::edit(
        "drag",
        "drag",
        opened.revision,
        EditPhase::Preview,
        operation.clone(),
    )) else {
        panic!("expected a preview of the linked target");
    };
    assert!(
        scene
            .transforms
            .iter()
            .any(|transform| transform.id == "alpha/T" && transform.pose.at == at)
    );
    let (_, committed) = fixture::scene_and_document(engine.commit_edit(fixture::commit(
        "drag",
        "drag",
        opened.revision,
        operation,
    )));
    assert_eq!(fixture::part_by_id(&committed, "alpha/T").pose.at, at);
    let Constraint::Framed {
        relation: ConstraintRelation::Offset { offset, rotation },
        ..
    } = &committed.constraints[0]
    else {
        panic!("expected the framed offset to survive the drag")
    };
    assert_eq!(*offset, Vec2 { x: 6.0, y: 6.0 });
    assert_eq!(rotation.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn a_cell_key_is_a_matrix_key_and_its_companion_is_not() {
    let (_, doc) = fixture(held_by_part_axes);
    assert!(boardstudio_core::is_matrix_key(&doc, KEY));
    assert!(!boardstudio_core::is_matrix_key(&doc, HELD));
    assert!(!boardstudio_core::is_matrix_key(&doc, "no-such-part"));
}
