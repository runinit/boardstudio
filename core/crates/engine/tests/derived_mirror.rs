use boardstudio_core_engine::CoreEngine;
use boardstudio_core_engine::model::{
    CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, Position, ProjectDoc, Vec2,
};
use serde_json::json;

fn document() -> ProjectDoc {
    let mut value = serde_json::to_value(ProjectDoc::empty("mirror", "Mirror")).unwrap();
    value["boards"] = json!([
        {"id":"source","name":"Source","partIds":["switch"],"outlineIds":[],"netIds":[],"thickness":1.6},
        {"id":"destination","name":"Destination","partIds":[],"outlineIds":[],"netIds":[],"thickness":1.6}
    ]);
    value["definitions"] = json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}
    ]);
    value["parts"] = json!([
        {"id":"switch","definitionId":"switch","reference":"SW1","pose":{"at":{"x":2.0,"y":3.0},"rotation":15.0},"side":"front"}
    ]);
    value["mirroredPairs"] = json!([{
        "id":"pair",
        "source":{"kind":"secondBoard","id":"source"},
        "destination":{"kind":"secondBoard","id":"destination"},
        "mapping":{"at":{"x":10.0,"y":0.0},"rotation":0.0},
        "overrides":[]
    }]);
    serde_json::from_value(value).unwrap()
}

fn accepted(reply: CoreReply) -> (ProjectDoc, Vec2) {
    let CoreReply::Scene {
        document, scene, ..
    } = reply
    else {
        panic!("Expected accepted scene, got {reply:?}");
    };
    let target = scene
        .transforms
        .iter()
        .find(|part| part.id == "4/pair/part/switch")
        .unwrap();
    assert_eq!(document.parts.len(), 1);
    (*document, target.pose.at)
}

fn move_part(engine: &mut CoreEngine, revision: u64, part: &str, x: f64) -> CoreReply {
    engine.handle(CoreRequest::Edit {
        id: "move".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "move".into(),
            phase: EditPhase::Commit,
            target_ids: vec![part.into()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: part.into(),
                    at: Vec2 { x, y: 3.0 },
                }],
            },
        },
    })
}

#[test]
fn linked_edit_refusal_and_seeded_undo_redo_keep_canonical_geometry_exact() {
    let mut engine = CoreEngine::new();
    let (mut canonical, initial) = accepted(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: document(),
    }));
    assert_eq!(initial, Vec2 { x: 8.0, y: 3.0 });
    let refusal = move_part(&mut engine, canonical.revision, "4/pair/part/switch", 3.0);
    assert!(matches!(refusal, CoreReply::Error { message, .. } if message.contains("is linked")));
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Error { .. }
    ));

    let mut seed = 0x84ba_318a_u32;
    let mut previous = initial;
    for _ in 0..40 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let x = f64::from(seed) / 65536.0;
        let before = canonical.clone();
        let (edited, geometry) = accepted(move_part(&mut engine, canonical.revision, "switch", x));
        assert_eq!(
            geometry,
            Vec2 {
                x: 10.0 - x,
                y: 3.0
            }
        );

        let (mut undone, geometry) =
            accepted(engine.handle(CoreRequest::Undo { id: "undo".into() }));
        assert_eq!(geometry, previous);
        undone.revision = before.revision;
        assert_eq!(undone, before);

        let (redone, geometry) = accepted(engine.handle(CoreRequest::Redo { id: "redo".into() }));
        let mut expected = edited;
        expected.revision = redone.revision;
        assert_eq!(redone, expected);
        previous = geometry;
        canonical = redone;
    }

    let saved = serde_json::to_vec(&canonical).unwrap();
    let (reopened, geometry) = accepted(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: serde_json::from_slice(&saved).unwrap(),
    }));
    assert_eq!(serde_json::to_vec(&reopened).unwrap(), saved);
    assert_eq!(geometry, previous);
}
