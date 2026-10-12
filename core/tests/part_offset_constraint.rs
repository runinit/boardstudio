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
        row_offsets: vec![],
        column_offsets: vec![],
        column_staggers: vec![],
        column_splays: vec![0.0, 0.0, 0.0],
        column_origins: vec![],
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
    Constraint::PartOffset {
        id: "held-key".into(),
        source_part_id: KEY.into(),
        target_part_id: HELD.into(),
        offset: Vec2 { x: 1.0, y: -4.0 },
        rotation: 0.0,
    }
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
    Constraint::Offset {
        id: "held-key".into(),
        source_part_id: KEY.into(),
        target_part_id: HELD.into(),
        offset: Vec2 { x: 1.0, y: -4.0 },
        rotation: 0.0,
    }
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
        (offset.x - 2.8660254037844386).abs() < 1e-9
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
