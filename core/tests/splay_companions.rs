//! A column splay turns every key in the column and carries its companions with it:
//! a companion keeps its pose relative to its key (offset and turn), whether it
//! follows the key or was authored. An explicit constraint keeps its own contract.
use boardstudio_core::{CoreEngine, model::*};

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
    let assembly = |id: &str, x: f64, y: f64| MatrixAssembly {
        id: id.into(),
        definition_id: "diode".into(),
        offset: Vec2 { x, y },
        rotation: None,
        side: None,
    };
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
        assemblies: vec![
            assembly("follow", 2.0, 5.0),
            assembly("authored", -3.0, 6.0),
            assembly("held", 1.0, -4.0),
        ],
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

fn fixture() -> (CoreEngine, ProjectDoc) {
    let mut doc = ProjectDoc::empty("splay-companions", "Splay companions");
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
    doc.constraints.push(Constraint::Offset {
        id: "held-key".into(),
        source_part_id: "matrix/m/r0c1".into(),
        target_part_id: "matrix/m/r0c1/held".into(),
        offset: Vec2 { x: 1.0, y: -4.0 },
        rotation: 0.0,
    });
    let (mut engine, _) = {
        let mut engine = CoreEngine::new();
        let (_, doc) = scene(engine.handle(CoreRequest::Open {
            id: "reopen".into(),
            document: doc,
        }));
        (engine, doc)
    };
    let (_, doc) = scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
    // Author one companion away from its generated offset, as a drag would.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: "matrix/m/r0c1/authored".into(),
                at: Vec2 { x: 20.0, y: 8.0 },
            }],
        },
    );
    (engine, doc)
}

#[test]
fn a_column_splay_carries_following_and_authored_companions_with_their_keys() {
    let (mut engine, before) = fixture();
    let key = "matrix/m/r0c1";
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
    let turned = part(&after, key).pose.rotation - part(&before, key).pose.rotation;
    assert!((turned - 30.0).abs() < 1e-9, "key turned {turned}");
    let mut failures = Vec::new();
    for name in ["follow", "authored"] {
        let id = format!("{key}/{name}");
        let (was_at, was_turn) = relative(&before, key, &id);
        let (now_at, now_turn) = relative(&after, key, &id);
        if (was_at.x - now_at.x).abs() > 1e-9
            || (was_at.y - now_at.y).abs() > 1e-9
            || (was_turn - now_turn).abs() > 1e-9
        {
            failures.push(format!(
                "{name}: key-relative {was_at:?} turn {was_turn} -> {now_at:?} turn {now_turn}"
            ));
        }
    }
    // A constrained companion stays where its constraint puts it: the offset is a
    // translation from the key and the turn is relative to the key's turn.
    let key_pose = part(&after, key).pose;
    let held = part(&after, "matrix/m/r0c1/held").pose;
    assert!(
        (held.at.x - (key_pose.at.x + 1.0)).abs() < 1e-9
            && (held.at.y - (key_pose.at.y - 4.0)).abs() < 1e-9
            && (held.rotation - key_pose.rotation).abs() < 1e-9,
        "constrained companion left its constraint: {held:?}"
    );
    assert!(
        failures.is_empty(),
        "companions that did not move with the splayed key:\n{}",
        failures.join("\n")
    );
}

#[test]
fn a_translation_carries_an_authored_companion_by_exactly_the_keys_delta() {
    let (mut engine, before) = fixture();
    let key = "matrix/m/r0c1";
    let authored = "matrix/m/r0c1/authored";
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrixPlacement {
            matrix_id: "m".into(),
            change: MatrixPlacementChange::ColumnStagger {
                column: 1,
                value: 3.7,
            },
        },
    );
    let (was_key, now_key) = (part(&before, key).pose, part(&after, key).pose);
    assert_ne!(was_key.at, now_key.at, "the stagger must move the key");
    let (was, now) = (part(&before, authored).pose, part(&after, authored).pose);
    // The same additions the carry always did, so saved poses do not shift by an ulp.
    assert_eq!(now.at.x, was.at.x + (now_key.at.x - was_key.at.x));
    assert_eq!(now.at.y, was.at.y + (now_key.at.y - was_key.at.y));
    assert_eq!(now.rotation, was.rotation);
}
