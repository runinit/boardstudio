//! Ticket 01: document admission rejects ambiguous identities and revision
//! exhaustion before any accepted state or history changes.
use boardstudio_core::CoreEngine;
use serde_json::{Value, json};

fn request(engine: &mut CoreEngine, input: Value) -> Value {
    serde_json::from_str(&engine.request(&input.to_string())).unwrap()
}

fn baseline() -> Value {
    let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "document-integrity",
        "Document integrity",
    ))
    .unwrap();
    document["definitions"] = json!([{
        "id": "def", "name": "Switch", "kind": "switch",
        "courtyard": [], "pads": []
    }]);
    document["parts"] = json!([
        {"id":"a","definitionId":"def","reference":"SW1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"},
        {"id":"b","definitionId":"def","reference":"SW2","pose":{"at":{"x":19,"y":0},"rotation":0},"side":"front"}
    ]);
    document["boards"] = json!([{
        "id":"board","name":"Board","outlineIds":[],"partIds":["a","b"],"netIds":[],
        "thickness":1.6
    }]);
    document["matrices"] = json!([{
        "id":"matrix","rows":1,"columns":2,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},
        "definitionId":"def","partIds":["a","b"],"boardId":"board"
    }]);
    document["nets"] = json!([{"id":"net","name":"Net","pins":[]}]);
    document["constraints"] = json!([{
        "kind":"offset","id":"constraint","sourcePartId":"a","targetPartId":"b",
        "offset":{"x":19,"y":0},"rotation":0
    }]);
    document
}

fn open(engine: &mut CoreEngine, id: &str, document: &Value) -> Value {
    request(engine, json!({"kind":"open","id":id,"document":document}))
}

fn snapshot(engine: &mut CoreEngine) -> Value {
    request(engine, json!({"kind":"snapshot","id":"snapshot"}))["document"].clone()
}

fn duplicate(document: &mut Value, collection: &str) {
    let entries = document[collection].as_array_mut().unwrap();
    let first = entries[0].clone();
    entries.push(first);
}

fn assert_error(reply: &Value) {
    assert_eq!(reply["kind"], "error", "expected an error, got {reply}");
}

#[test]
fn duplicate_identity_in_each_collection_is_rejected_on_open() {
    for collection in [
        "parts",
        "definitions",
        "boards",
        "matrices",
        "nets",
        "constraints",
    ] {
        let mut document = baseline();
        duplicate(&mut document, collection);
        let mut engine = CoreEngine::new();
        let reply = open(&mut engine, "duplicate", &document);
        assert_error(&reply);
        assert!(
            reply["message"].as_str().unwrap().contains("Duplicate"),
            "{collection}: {}",
            reply["message"]
        );
        assert_eq!(snapshot(&mut engine)["revision"], 0);
    }
}

#[test]
fn duplicate_identity_is_rejected_on_replacement_and_preserves_accepted_state() {
    for collection in [
        "parts",
        "definitions",
        "boards",
        "matrices",
        "nets",
        "constraints",
    ] {
        let mut engine = CoreEngine::new();
        assert_eq!(open(&mut engine, "open", &baseline())["kind"], "scene");
        let accepted = snapshot(&mut engine);

        let mut replacement = baseline();
        replacement["revision"] = accepted["revision"].clone();
        duplicate(&mut replacement, collection);
        let reply = request(
            &mut engine,
            json!({
                "kind":"edit","id":"replace",
                "command":{
                    "baseRevision":accepted["revision"],"transactionId":"replace","phase":"commit",
                    "targetIds":[],"operation":{"kind":"replace-document","document":replacement}
                }
            }),
        );
        assert_error(&reply);
        assert!(
            reply["message"].as_str().unwrap().contains("Duplicate"),
            "{collection}: {}",
            reply["message"]
        );
        assert_eq!(snapshot(&mut engine), accepted, "{collection} state moved");
    }
}

#[test]
fn unique_identities_still_open_and_move_parts() {
    let mut engine = CoreEngine::new();
    let mut document = baseline();
    document["constraints"] = json!([]);
    assert_eq!(open(&mut engine, "open", &document)["kind"], "scene");
    let moved = request(
        &mut engine,
        json!({
            "kind":"edit","id":"move","command":{
                "baseRevision":0,"transactionId":"move","phase":"commit","targetIds":["b"],
                "operation":{"kind":"move-parts","positions":[
                    {"id":"a","at":{"x":5,"y":6}},{"id":"b","at":{"x":24,"y":6}}
                ]}
            }
        }),
    );
    assert_eq!(moved["kind"], "scene");
    assert_eq!(moved["scene"]["revision"], 1);
    assert_eq!(moved["document"]["parts"][1]["pose"]["at"]["x"], 24.0);
}

#[test]
fn max_revision_open_is_rejected_without_replacing_the_accepted_document() {
    let mut engine = CoreEngine::new();
    assert_eq!(open(&mut engine, "open", &baseline())["kind"], "scene");
    let accepted = snapshot(&mut engine);

    let mut exhausted = baseline();
    exhausted["revision"] = json!(u64::MAX);
    let reply = open(&mut engine, "exhausted", &exhausted);
    assert_error(&reply);
    assert!(
        reply["message"]
            .as_str()
            .unwrap()
            .contains("revision is exhausted")
    );
    assert_eq!(snapshot(&mut engine), accepted);
}

#[test]
fn max_minus_one_advances_to_max_then_refuses_the_next_advance_atomically() {
    let mut engine = CoreEngine::new();
    let mut document = baseline();
    document["revision"] = json!(u64::MAX - 1);
    assert_eq!(open(&mut engine, "open", &document)["kind"], "scene");

    let at_max = request(
        &mut engine,
        json!({
            "kind":"edit","id":"advance","command":{
                "baseRevision":u64::MAX - 1,"transactionId":"advance","phase":"commit",
                "targetIds":["board"],
                "operation":{"kind":"set-wiring-mode","boardId":"board","mode":"direct"}
            }
        }),
    );
    assert_eq!(at_max["kind"], "scene");
    assert_eq!(at_max["scene"]["revision"], u64::MAX);
    let accepted = snapshot(&mut engine);
    assert_eq!(accepted["revision"], u64::MAX);

    let refused = request(
        &mut engine,
        json!({
            "kind":"edit","id":"refused","command":{
                "baseRevision":u64::MAX,"transactionId":"refused","phase":"commit",
                "targetIds":["board"],
                "operation":{"kind":"set-wiring-mode","boardId":"board","mode":"matrix"}
            }
        }),
    );
    assert_error(&refused);
    assert!(
        refused["message"]
            .as_str()
            .unwrap()
            .contains("revision is exhausted")
    );
    assert_eq!(snapshot(&mut engine), accepted);
}

#[test]
fn max_revision_undo_and_redo_are_refused_with_their_history_intact() {
    // Undo side: a commit lands at MAX, leaving a nonempty undo stack.
    let mut engine = CoreEngine::new();
    let mut document = baseline();
    document["revision"] = json!(u64::MAX - 1);
    assert_eq!(open(&mut engine, "open", &document)["kind"], "scene");
    assert_eq!(
        request(
            &mut engine,
            json!({
                "kind":"edit","id":"commit","command":{
                    "baseRevision":u64::MAX - 1,"transactionId":"commit","phase":"commit",
                    "targetIds":["board"],
                    "operation":{"kind":"set-wiring-mode","boardId":"board","mode":"direct"}
                }
            })
        )["scene"]["revision"],
        u64::MAX
    );
    let accepted = snapshot(&mut engine);
    let undo = request(&mut engine, json!({"kind":"undo","id":"undo"}));
    assert_error(&undo);
    assert!(
        undo["message"]
            .as_str()
            .unwrap()
            .contains("revision is exhausted")
    );
    assert_eq!(snapshot(&mut engine), accepted);

    // Redo side: undo from MAX-1 lands at MAX, leaving a nonempty redo stack.
    let mut engine = CoreEngine::new();
    let mut document = baseline();
    document["revision"] = json!(u64::MAX - 2);
    assert_eq!(open(&mut engine, "open", &document)["kind"], "scene");
    request(
        &mut engine,
        json!({
            "kind":"edit","id":"commit","command":{
                "baseRevision":u64::MAX - 2,"transactionId":"commit","phase":"commit",
                "targetIds":["board"],
                "operation":{"kind":"set-wiring-mode","boardId":"board","mode":"direct"}
            }
        }),
    );
    let undone = request(&mut engine, json!({"kind":"undo","id":"undo"}));
    assert_eq!(undone["scene"]["revision"], u64::MAX);
    let accepted = snapshot(&mut engine);
    let redo = request(&mut engine, json!({"kind":"redo","id":"redo"}));
    assert_error(&redo);
    assert!(
        redo["message"]
            .as_str()
            .unwrap()
            .contains("revision is exhausted")
    );
    assert_eq!(snapshot(&mut engine), accepted);
}

#[test]
fn remap_review_at_max_refuses_before_clearing_a_matching_protected_handoff() {
    let mut document = baseline();
    document["revision"] = json!(u64::MAX - 1);
    document["hardware"] = json!({
        "boards":[{
            "boardId":"board",
            "protectedHandoff":{"fingerprint":"reviewed-fingerprint","revision":0,"assignments":{}}
        }]
    });
    let mut engine = CoreEngine::new();
    assert_eq!(open(&mut engine, "open", &document)["kind"], "scene");

    let committed = request(
        &mut engine,
        json!({
            "kind":"edit","id":"commit","command":{
                "baseRevision":u64::MAX - 1,"transactionId":"commit","phase":"commit",
                "targetIds":["board"],
                "operation":{"kind":"set-wiring-mode","boardId":"board","mode":"direct"}
            }
        }),
    );
    assert_eq!(committed["scene"]["revision"], u64::MAX);
    let accepted = snapshot(&mut engine);
    assert_eq!(
        accepted["hardware"]["boards"][0]["protectedHandoff"]["fingerprint"],
        "reviewed-fingerprint"
    );

    let refused = request(
        &mut engine,
        json!({
            "kind":"review-electrical-remap","id":"remap","baseRevision":u64::MAX,
            "boardId":"board","expectedFingerprint":"reviewed-fingerprint"
        }),
    );
    assert_error(&refused);
    assert!(
        refused["message"]
            .as_str()
            .unwrap()
            .contains("revision is exhausted")
    );
    assert_eq!(snapshot(&mut engine), accepted);
}

#[test]
fn refused_open_preserves_nonempty_history() {
    let mut engine = CoreEngine::new();
    assert_eq!(open(&mut engine, "open", &baseline())["kind"], "scene");
    assert_eq!(
        request(
            &mut engine,
            json!({
                "kind":"edit","id":"commit","command":{
                    "baseRevision":0,"transactionId":"commit","phase":"commit",
                    "targetIds":["board"],
                    "operation":{"kind":"set-wiring-mode","boardId":"board","mode":"direct"}
                }
            })
        )["scene"]["revision"],
        1
    );

    let mut duplicate_parts = baseline();
    duplicate(&mut duplicate_parts, "parts");
    assert_error(&open(&mut engine, "duplicate", &duplicate_parts));

    let accepted = snapshot(&mut engine);
    assert_eq!(accepted["revision"], 1);
    let undone = request(&mut engine, json!({"kind":"undo","id":"undo"}));
    assert_eq!(undone["kind"], "scene");
    assert_eq!(undone["scene"]["revision"], 2);
    assert!(undone["document"].get("hardware").is_none());
}
