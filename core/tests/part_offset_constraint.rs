//! A `part-offset` constraint holds its target at a fixed distance and angle along its
//! source part's own axes, so a companion held to a key keeps its pose in the key's frame
//! whenever the key turns. An `offset` constraint keeps its board-axes meaning.
use boardstudio_core::{CoreEngine, model::*};

const KEY: &str = "matrix/m/r0c1";
const HELD: &str = "matrix/m/r0c1/held";

fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}

fn edit(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    scene(engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }))
}

fn matrix() -> Matrix {
    let cell = |column: u32| MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        row: 0,
        column,
        enabled: true,
        definition_id: None,
        variant: None,
        offset: None,
        rotation: None,
        assemblies: vec![MatrixAssembly {
            id: "held".into(),
            definition_id: "diode".into(),
            offset: Vec2 { x: 1.0, y: -4.0 },
            rotation: None,
            side: None,
        }],
        assemblies_local: None,
    };
    Matrix {
        id: "m".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: None,
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![
            boardstudio_core::model::ColumnLayout::default(),
            boardstudio_core::model::ColumnLayout::default(),
            boardstudio_core::model::ColumnLayout::default(),
        ],
        anchor: None,
        cells: vec![cell(0), cell(1), cell(2)],
    }
}

fn part<'a>(doc: &'a ProjectDoc, id: &str) -> &'a Part {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
}

/// The companion's pose in its key's frame: where the key's own turn leaves it.
fn relative(doc: &ProjectDoc, key: &str, companion: &str) -> (Vec2, f64) {
    let key = part(doc, key).pose;
    let companion = part(doc, companion).pose;
    let (sin, cos) = (-key.rotation).to_radians().sin_cos();
    let (dx, dy) = (companion.at.x - key.at.x, companion.at.y - key.at.y);
    (
        Vec2 {
            x: dx * cos - dy * sin,
            y: dx * sin + dy * cos,
        },
        companion.rotation - key.rotation,
    )
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
    let (_, doc) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let (_, mut doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: matrix(),
            definitions: None,
        },
    );
    doc.constraints.push(constraint());
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: doc,
    }));
    let (_, doc) = scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
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
    let (was_at, was_turn) = relative(before, KEY, HELD);
    let (now_at, now_turn) = relative(after, KEY, HELD);
    assert!(
        (was_at.x - now_at.x).abs() < 1e-9
            && (was_at.y - now_at.y).abs() < 1e-9
            && (was_turn - now_turn).abs() < 1e-9,
        "key-relative {was_at:?} turn {was_turn} -> {now_at:?} turn {now_turn}"
    );
}

#[test]
fn a_part_offset_companion_keeps_its_key_frame_pose_through_a_column_splay() {
    let (mut engine, before) = fixture(held_by_part_axes);
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
    let turned = part(&after, KEY).pose.rotation - part(&before, KEY).pose.rotation;
    assert!((turned - 30.0).abs() < 1e-9, "key turned {turned}");
    assert_key_frame_pose_kept(&before, &after);
}

#[test]
fn a_part_offset_companion_keeps_its_key_frame_pose_through_a_following_splay() {
    let (mut engine, before) = fixture(held_by_part_axes);
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrixSplay {
            matrix_id: "m".into(),
            column: 1,
            change: MatrixSplayChange::Angle {
                angle: 30.0,
                affect: MatrixSplayAffect::Following,
            },
        },
    );
    assert_key_frame_pose_kept(&before, &after);
}

#[test]
fn a_part_offset_companion_keeps_its_key_frame_pose_through_a_matrix_rotation() {
    let (mut engine, before) = fixture(held_by_part_axes);
    let mut turned = before.matrices[0].clone();
    turned.rotation = Some(25.0);
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrix {
            matrix: turned,
            definitions: None,
        },
    );
    assert!(
        (part(&after, KEY).pose.rotation - part(&before, KEY).pose.rotation).abs() > 1e-6,
        "the matrix rotation must turn the key"
    );
    assert_key_frame_pose_kept(&before, &after);
}

#[test]
fn a_part_offset_companion_keeps_its_key_frame_pose_when_its_key_moves() {
    let (mut engine, before) = fixture(held_by_part_axes);
    let key_at = part(&before, KEY).pose.at;
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: KEY.into(),
                at: Vec2 {
                    x: key_at.x + 5.0,
                    y: key_at.y + 2.0,
                },
            }],
        },
    );
    assert!(
        (part(&after, KEY).pose.at.x - (key_at.x + 5.0)).abs() < 1e-9,
        "the key did not move"
    );
    assert_key_frame_pose_kept(&before, &after);
}

#[test]
fn dragging_a_part_offset_companion_retunes_its_offset_along_the_source_axes() {
    let (mut engine, before) = fixture(held_by_part_axes);
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
    // Drag the companion 3 mm along the board's X axis. The key is turned 30 degrees, so
    // the stored offset must move by that drag rotated back into the key's frame:
    // (3 cos 30, -3 sin 30) on top of (1, -4).
    let start = part(&splayed, HELD).pose.at;
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
    assert!(
        (part(&after, HELD).pose.at.x - target.x).abs() < 1e-9
            && (part(&after, HELD).pose.at.y - target.y).abs() < 1e-9,
        "companion did not land on the drag: {:?}",
        part(&after, HELD).pose.at
    );
    let Some(Constraint::PartOffset { offset, .. }) = after
        .constraints
        .iter()
        .find(|constraint| constraint.target() == HELD)
    else {
        panic!("the part-offset constraint was lost");
    };
    assert!(
        (offset.x - (1.0 + 2.598_076_211_353_316)).abs() < 1e-9
            && (offset.y - (-4.0 - 1.5)).abs() < 1e-9,
        "offset {offset:?} is not the drag in the key's axes"
    );
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
) -> CoreReply {
    engine.handle(CoreRequest::Edit {
        id: "convert".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "convert".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::ConvertOffsetConstraint {
                constraint_id: constraint_id.into(),
                to,
            },
        },
    })
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
fn converting_a_board_offset_to_part_axes_moves_no_part_and_keeps_the_id() {
    let (mut engine, before) = splayed(held_by_board_axes);
    let (_, after) = scene(convert(&mut engine, &before, "held-key", OffsetAxes::Part));
    assert_every_part_kept(&before, &after);
    let Constraint::PartOffset {
        id,
        offset,
        rotation,
        ..
    } = held_constraint(&after)
    else {
        panic!("the constraint did not become part-offset");
    };
    assert_eq!(id, "held-key");
    // (1, -4) on board axes is (-1.134, -3.964) in the key's frame at a 30 degree turn.
    assert!(
        (offset.x - -1.1339745962155614).abs() < 1e-9
            && (offset.y - -3.9641016151377544).abs() < 1e-9
            && rotation.abs() < 1e-9,
        "offset {offset:?} turn {rotation}"
    );
}

#[test]
fn converting_a_part_offset_to_board_axes_moves_no_part_and_keeps_the_id() {
    let (mut engine, before) = splayed(held_by_part_axes);
    let (_, after) = scene(convert(&mut engine, &before, "held-key", OffsetAxes::Board));
    assert_every_part_kept(&before, &after);
    let Constraint::Offset {
        id,
        offset,
        rotation,
        ..
    } = held_constraint(&after)
    else {
        panic!("the constraint did not become offset");
    };
    assert_eq!(id, "held-key");
    // (1, -4) in the key's frame is (2.866, -2.964) on board axes at a 30 degree turn.
    assert!(
        (offset.x - 2.866_025_403_784_439).abs() < 1e-9
            && (offset.y - -2.9641016151377544).abs() < 1e-9
            && rotation.abs() < 1e-9,
        "offset {offset:?} turn {rotation}"
    );
}

#[test]
fn converting_an_unknown_constraint_is_refused_and_changes_nothing() {
    let (mut engine, before) = splayed(held_by_part_axes);
    let CoreReply::Error { message, .. } = convert(
        &mut engine,
        &before,
        "no-such-constraint",
        OffsetAxes::Board,
    ) else {
        panic!("an unknown constraint must be refused");
    };
    assert!(message.contains("no-such-constraint"), "{message}");
    let (_, now) = scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
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
        let CoreReply::Error { message, .. } = convert(&mut engine, &before, "held-key", to) else {
            panic!("a mirror has no offset axes to convert to {to:?}");
        };
        assert!(message.contains("mirror"), "{message}");
        let (_, now) = scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
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
        boardstudio_core::migrate::CURRENT_VERSION
    );
}

#[test]
fn previewing_a_drag_of_a_constrained_companion_leaves_its_stored_offset_untouched() {
    for (kind, constraint) in [
        ("offset", held_by_board_axes as fn() -> Constraint),
        ("part-offset", held_by_part_axes as fn() -> Constraint),
    ] {
        let (mut engine, before) = splayed(constraint);
        let start = part(&before, HELD).pose.at;
        let _ = engine.handle(CoreRequest::Edit {
            id: "preview".into(),
            command: EditCommand {
                base_revision: before.revision,
                transaction_id: "drag".into(),
                phase: EditPhase::Preview,
                target_ids: vec![],
                operation: EditOperation::MoveParts {
                    positions: vec![Position {
                        id: HELD.into(),
                        at: Vec2 {
                            x: start.x + 3.0,
                            y: start.y,
                        },
                    }],
                },
            },
        });
        let (_, now) = scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
        assert_eq!(
            now.constraints, before.constraints,
            "{kind} was changed by a preview"
        );
        assert_eq!(
            part(&now, HELD).pose,
            part(&before, HELD).pose,
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

/// A board-axes drag lands the stored offset on the board's axes, whatever the key's turn.
#[test]
fn dragging_a_board_axes_companion_retunes_its_offset_on_the_board_axes() {
    let (mut engine, splayed) = splayed(held_by_board_axes);
    let turn = part(&splayed, KEY).pose.rotation;
    assert!(
        (turn - 30.0).abs() < 1e-9,
        "the key must be turned, found {turn}"
    );
    let start = part(&splayed, HELD).pose.at;
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
    let landed = part(&after, HELD).pose.at;
    assert!(
        (landed.x - target.x).abs() < 1e-9 && (landed.y - target.y).abs() < 1e-9,
        "companion did not land on the drag: {landed:?}"
    );
    let Some(Constraint::Offset {
        offset, rotation, ..
    }) = after
        .constraints
        .iter()
        .find(|constraint| constraint.target() == HELD)
    else {
        panic!("the board-axes constraint was lost");
    };
    // The drag is added as it is: (1, -4) + (3, 0). The 30 degree key turn plays no part.
    assert!(
        (offset.x - 4.0).abs() < 1e-9 && (offset.y - -4.0).abs() < 1e-9,
        "offset {offset:?} is not the drag on the board's axes"
    );
    assert_eq!(*rotation, 0.0);
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
            let start = part(&before, HELD).pose.at;
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

/// A mirrored pair of layouts. The left layout owns a free diode, held to a left key that is
/// turned `left_rotation` degrees by an offset constraint along `axes`. The right layout's
/// mirror of that diode is derived by Core.
fn mirrored_pair_with_held_left_component(
    left_rotation: f64,
    axes: OffsetAxes,
) -> (CoreEngine, ProjectDoc) {
    let mut doc = ProjectDoc::empty("pair", "Pair");
    doc.definitions = serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]},
        {"id":"diode","name":"Diode","kind":"passive","courtyard":[],"pads":[]},
    ]))
    .unwrap();
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["left-part".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.parts.push(Part {
        id: "left-part".into(),
        definition_id: "diode".into(),
        reference: "D1".into(),
        pose: Pose2 {
            at: Vec2 { x: 2.0, y: 5.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    let mut engine = CoreEngine::new();
    let (_, doc) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let left = Matrix {
        id: "left-matrix".into(),
        name: None,
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: Some("board".into()),
        mirror: None,
        rotation: Some(left_rotation),
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![boardstudio_core::model::ColumnLayout::default()],
        anchor: None,
        cells: vec![MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row: 0,
            column: 0,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        }],
    };
    let layout = |id: &str, matrix_id: &str, part_ids: Vec<String>, mirror_link| Layout {
        id: id.into(),
        name: id.into(),
        board_id: "board".into(),
        matrix_ids: vec![matrix_id.into()],
        part_ids,
        mirror_link,
    };
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::CreateMirroredPair {
            left: layout("left-layout", "left-matrix", vec![], None),
            right: layout(
                "right-layout",
                "right-matrix",
                vec![],
                Some(LayoutMirrorLink {
                    correspondences: vec![],
                    exceptions: vec![],
                    source_id: "left-layout".into(),
                    axis_x: 10.0,
                    split_gap: None,
                }),
            ),
            matrix: Box::new(left),
            definitions: None,
        },
    );
    // The diode joins the left layout after the pair exists, which derives its mirror.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: layout("left-layout", "left-matrix", vec!["left-part".into()], None),
        },
    );
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetConstraint {
            constraint: held_by(
                axes,
                "left-held",
                "matrix/left-matrix/r0c0",
                "left-part",
                Vec2 { x: 1.0, y: -4.0 },
                0.0,
            ),
        },
    );
    (engine, doc)
}

#[test]
fn dragging_the_mirrored_half_retunes_a_held_part_offset_in_its_own_key_frame() {
    let (mut engine, before) = mirrored_pair_with_held_left_component(30.0, OffsetAxes::Part);
    let left_key = part(&before, "matrix/left-matrix/r0c0").pose;
    assert!(
        (left_key.rotation - 30.0).abs() < 1e-9,
        "the left key must be turned 30 degrees, found {}",
        left_key.rotation
    );
    let right = "right-layout::mirror::left-part";
    let start = part(&before, right).pose.at;
    let requested = Vec2 {
        x: start.x + 3.0,
        y: start.y,
    };
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: right.into(),
                at: requested,
            }],
        },
    );
    // The mirror of the requested right pose across the axis at x = 10.
    let mirrored = Vec2 {
        x: 2.0 * 10.0 - requested.x,
        y: requested.y,
    };
    let landed = part(&after, "left-part").pose.at;
    assert!(
        (landed.x - mirrored.x).abs() < 1e-9 && (landed.y - mirrored.y).abs() < 1e-9,
        "left component at {landed:?}, expected the mirror {mirrored:?}"
    );
    // The left component's world move, rotated into the left key's frame at 30 degrees:
    // (dx cos 30 + dy sin 30, -dx sin 30 + dy cos 30).
    let left_start = part(&before, "left-part").pose.at;
    let (dx, dy) = (mirrored.x - left_start.x, mirrored.y - left_start.y);
    let (c, s) = (30.0_f64.to_radians().cos(), 30.0_f64.to_radians().sin());
    let Some(Constraint::PartOffset { offset, .. }) = after
        .constraints
        .iter()
        .find(|constraint| constraint.target() == "left-part")
    else {
        panic!("the held left component lost its part-offset constraint");
    };
    assert!(
        (offset.x - (1.0 + dx * c + dy * s)).abs() < 1e-9
            && (offset.y - (-4.0 - dx * s + dy * c)).abs() < 1e-9,
        "offset {offset:?} is not the mirrored drag in the key's axes"
    );
}

#[test]
fn dragging_the_mirrored_half_retunes_a_held_board_offset_on_the_board_axes() {
    let (mut engine, before) = mirrored_pair_with_held_left_component(30.0, OffsetAxes::Board);
    let left_key = part(&before, "matrix/left-matrix/r0c0").pose;
    assert!(
        (left_key.rotation - 30.0).abs() < 1e-9,
        "the left key must be turned 30 degrees, found {}",
        left_key.rotation
    );
    let right = "right-layout::mirror::left-part";
    let start = part(&before, right).pose.at;
    let requested = Vec2 {
        x: start.x + 3.0,
        y: start.y,
    };
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: right.into(),
                at: requested,
            }],
        },
    );
    // Dragging the mirror 3 mm along +X moves the left component 3 mm along -X.
    let left_start = part(&before, "left-part").pose.at;
    let landed = part(&after, "left-part").pose.at;
    assert!(
        (landed.x - (left_start.x - 3.0)).abs() < 1e-9 && (landed.y - left_start.y).abs() < 1e-9,
        "left component at {landed:?}, expected {:?}",
        Vec2 {
            x: left_start.x - 3.0,
            y: left_start.y
        }
    );
    let Some(Constraint::Offset { offset, .. }) = after
        .constraints
        .iter()
        .find(|constraint| constraint.target() == "left-part")
    else {
        panic!("the held left component lost its offset constraint");
    };
    // Board axes: (1, -4) + (-3, 0). The 30 degree key turn plays no part.
    assert!(
        (offset.x - -2.0).abs() < 1e-9 && (offset.y - -4.0).abs() < 1e-9,
        "offset {offset:?} is not the mirrored drag on the board's axes"
    );
}

/// Holding the right mirror to its own key with a turn of 10 degrees settles that mirror at
/// -20 degrees (its key is at -30), so the left component is back-solved to 180 - (-20) = 200
/// degrees. Its key stands at 30, so its constraint must now store a turn of 170.
///
/// The mirror is a genuine independent part, so it keeps ADR-0010's half turn; the stored
/// heading is normalized to `[0, 360)` by the shared mirror-heading helper, i.e. 340 for
/// -20, matching the long-standing `derive::pose` convention.
#[test]
fn a_held_mirror_turns_the_held_left_component_and_retunes_its_stored_turn() {
    for axes in [OffsetAxes::Board, OffsetAxes::Part] {
        let (mut engine, before) = mirrored_pair_with_held_left_component(30.0, axes);
        assert!((part(&before, "matrix/right-matrix/r0c0").pose.rotation - -30.0).abs() < 1e-9);
        let (_, after) = edit(
            &mut engine,
            &before,
            EditOperation::SetConstraint {
                constraint: held_by(
                    OffsetAxes::Board,
                    "right-held",
                    "matrix/right-matrix/r0c0",
                    "right-layout::mirror::left-part",
                    Vec2 { x: 1.0, y: 4.0 },
                    10.0,
                ),
            },
        );
        let right = part(&after, "right-layout::mirror::left-part")
            .pose
            .rotation;
        // Modulo a full turn: the shared helper stores -20 normalized as 340.
        assert!(
            (right - -20.0).rem_euclid(360.0) < 1e-9,
            "{axes:?}: right turn {right}"
        );
        let left = part(&after, "left-part").pose.rotation;
        assert!((left - 200.0).abs() < 1e-9, "{axes:?}: left turn {left}");
        let Some(Constraint::Offset { rotation, .. } | Constraint::PartOffset { rotation, .. }) =
            after
                .constraints
                .iter()
                .find(|constraint| constraint.target() == "left-part")
        else {
            panic!("the held left component lost its constraint");
        };
        assert!(
            (rotation - 170.0).abs() < 1e-9,
            "{axes:?}: stored turn {rotation}"
        );
    }
}

#[test]
fn a_cell_key_is_a_matrix_key_and_its_companion_is_not() {
    let (_, doc) = fixture(held_by_part_axes);
    assert!(boardstudio_core::is_matrix_key(&doc, KEY));
    assert!(!boardstudio_core::is_matrix_key(&doc, HELD));
    assert!(!boardstudio_core::is_matrix_key(&doc, "no-such-part"));
}
