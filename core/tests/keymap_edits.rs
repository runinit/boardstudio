use boardstudio_core::{CoreEngine, model::*};
use serde_json::{Value, json};

fn fixture() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("keys", "Keys");
    doc.definitions.push(
        serde_json::from_value(
            json!({"id":"switch","name":"Switch","kind":"switch","pads":[],"courtyard":[]}),
        )
        .unwrap(),
    );
    doc.parts.push(serde_json::from_value(json!({"id":"matrix/m/r0c0","definitionId":"switch","reference":"SW1","side":"front","pose":{"at":{"x":0,"y":0},"rotation":0}})).unwrap());
    doc.matrices.push(serde_json::from_value(json!({"id":"m","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"switch","partIds":["matrix/m/r0c0"],"boardId":"board"})).unwrap());
    doc.boards.push(serde_json::from_value(json!({"id":"board","name":"Board","thickness":1.6,"partIds":["matrix/m/r0c0"],"outlineIds":[],"netIds":[]})).unwrap());
    doc.boards.push(serde_json::from_value(json!({"id":"other","name":"Other","thickness":1.6,"partIds":[],"outlineIds":[],"netIds":[]})).unwrap());
    doc
}

fn open(doc: ProjectDoc) -> CoreEngine {
    let mut engine = CoreEngine::default();
    assert!(matches!(
        engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: doc
        }),
        CoreReply::Scene { .. }
    ));
    engine
}

fn edit(engine: &mut CoreEngine, revision: u64, operation: Value) -> CoreReply {
    serde_json::from_str(&engine.request(&json!({"id":"edit","kind":"edit","command":{"baseRevision":revision,"transactionId":"keymap","phase":"commit","targetIds":[],"operation":operation}}).to_string())).unwrap()
}

fn document(reply: CoreReply) -> ProjectDoc {
    let CoreReply::Scene { document, .. } = reply else {
        panic!("expected committed scene: {reply:?}")
    };
    document
}

#[test]
fn bindings_create_defaults_preserve_geometry_and_support_history() {
    let original = fixture();
    let mut engine = open(original.clone());
    let doc = document(edit(
        &mut engine,
        0,
        json!({"kind":"set-key-binding","boardId":"board","keyId":"matrix/m/r0c0","binding":"&kp A"}),
    ));
    assert_eq!(doc.parts, original.parts);
    assert_eq!(
        doc.hardware.as_ref().unwrap().boards[0].key_bindings["matrix/m/r0c0"],
        "&kp A"
    );
    assert_eq!(doc.revision, 1);
    let undone = document(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert!(undone.hardware.is_none());
    let redone = document(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(
        redone.hardware.unwrap().boards[0].key_bindings["matrix/m/r0c0"],
        "&kp A"
    );
}

#[test]
fn field_edits_preserve_other_keys_boards_and_handoff_protection() {
    let mut doc = fixture();
    doc.hardware = Some(serde_json::from_value(json!({"boards":[{"boardId":"board","keyBindings":{"existing":"&kp B"},"locks":{"row":"P1"},"protectedHandoff":{"fingerprint":"pins","revision":5,"assignments":{"row":"P1"}}},{"boardId":"other","keyBindings":{"other-key":"&kp Z"}}]})).unwrap());
    let before = doc.hardware.clone().unwrap();
    let mut engine = open(doc);
    let next = document(edit(
        &mut engine,
        0,
        json!({"kind":"set-key-binding","boardId":"board","keyId":"matrix/m/r0c0","binding":"&kp A"}),
    ));
    let hardware = next.hardware.unwrap();
    assert_eq!(
        hardware.boards[0].protected_handoff,
        before.boards[0].protected_handoff
    );
    assert_eq!(hardware.boards[0].locks, before.boards[0].locks);
    assert_eq!(hardware.boards[0].key_bindings["existing"], "&kp B");
    assert_eq!(hardware.boards[1], before.boards[1]);
}

#[test]
fn matrix_and_key_edits_merge_fields_and_distinguish_blank_from_inherited() {
    let mut engine = open(fixture());
    let doc = document(edit(
        &mut engine,
        0,
        json!({"kind":"set-matrix-keycaps","matrixId":"m","change":{"kind":"profile","value":"dsa"}}),
    ));
    assert_eq!(
        doc.keycaps.as_ref().unwrap().matrices["m"].wall_thickness,
        1.2
    );
    let doc = document(edit(
        &mut engine,
        1,
        json!({"kind":"set-matrix-keycaps","matrixId":"m","change":{"kind":"wall-thickness","value":1.5}}),
    ));
    assert_eq!(
        doc.keycaps.as_ref().unwrap().matrices["m"].profile,
        Some(KeycapProfile::Dsa)
    );
    document(edit(
        &mut engine,
        2,
        json!({"kind":"set-keycap-key","keyId":"matrix/m/r0c0","change":{"kind":"color","value":"#ff0000"}}),
    ));
    let doc = document(edit(
        &mut engine,
        3,
        json!({"kind":"set-keycap-key","keyId":"matrix/m/r0c0","change":{"kind":"legend","value":""}}),
    ));
    let settings = &doc.keycaps.as_ref().unwrap().keys["matrix/m/r0c0"];
    assert_eq!(settings.legend.as_deref(), Some(""));
    assert_eq!(settings.color.as_deref(), Some("#ff0000"));
    let doc = document(edit(
        &mut engine,
        4,
        json!({"kind":"set-keycap-key","keyId":"matrix/m/r0c0","change":{"kind":"legend","value":null}}),
    ));
    let settings = &doc.keycaps.as_ref().unwrap().keys["matrix/m/r0c0"];
    assert!(settings.legend.is_none());
    assert_eq!(settings.color.as_deref(), Some("#ff0000"));
}

#[test]
fn invalid_targets_values_and_stale_revisions_are_atomic() {
    let mut engine = open(fixture());
    for operation in [
        json!({"kind":"set-key-binding","boardId":"other","keyId":"matrix/m/r0c0","binding":"&kp A"}),
        json!({"kind":"set-matrix-keycaps","matrixId":"missing","change":{"kind":"profile","value":"dsa"}}),
        json!({"kind":"set-keycap-board","boardId":"board","change":{"kind":"clearance","value":-1}}),
        json!({"kind":"set-keycap-board","boardId":"board","change":{"kind":"color","value":"red"}}),
        json!({"kind":"set-keycap-key","keyId":"matrix/m/r0c0","change":{"kind":"units","value":{"x":0,"y":1}}}),
        json!({"kind":"set-keycap-key","keyId":"missing","change":{"kind":"legend","value":"A"}}),
    ] {
        assert!(matches!(
            edit(&mut engine, 0, operation),
            CoreReply::Error { .. }
        ));
        let snapshot = document(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into(),
        }));
        assert_eq!(snapshot.revision, 0);
        assert!(snapshot.keycaps.is_none());
        assert!(snapshot.hardware.is_none());
    }
    document(edit(
        &mut engine,
        0,
        json!({"kind":"set-keycap-board","boardId":"board","change":{"kind":"color","value":"#123456"}}),
    ));
    assert!(matches!(
        edit(
            &mut engine,
            0,
            json!({"kind":"set-keycap-board","boardId":"board","change":{"kind":"clearance","value":1}})
        ),
        CoreReply::Error { .. }
    ));
}

#[test]
fn encoder_push_bindings_use_the_same_membership_and_merge_path() {
    let mut doc = fixture();
    doc.definitions.push(
        serde_json::from_value(
            json!({"id":"encoder","name":"Encoder","kind":"encoder","pads":[],"courtyard":[]}),
        )
        .unwrap(),
    );
    doc.parts.push(serde_json::from_value(json!({"id":"encoder","definitionId":"encoder","reference":"RE1","side":"front","pose":{"at":{"x":40,"y":0},"rotation":0}})).unwrap());
    doc.boards[0].part_ids.push("encoder".into());
    let mut engine = open(doc);
    let next = document(edit(
        &mut engine,
        0,
        json!({"kind":"set-key-binding","boardId":"board","keyId":"encoder/push","binding":"&kp MUTE"}),
    ));
    assert_eq!(
        next.hardware.unwrap().boards[0].key_bindings["encoder/push"],
        "&kp MUTE"
    );
    assert!(matches!(
        edit(
            &mut engine,
            1,
            json!({"kind":"set-key-binding","boardId":"other","keyId":"encoder/push","binding":"&kp MUTE"})
        ),
        CoreReply::Error { .. }
    ));
}
