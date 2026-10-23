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
fn grouped_v7_records_and_layout_membership_round_trip_through_core() {
    let mut document = baseline();
    document["formatVersion"] = json!(7);
    document["constraints"] = json!([]);
    document["matrices"][0]["columnLayout"] = json!([
        {"rotation": 90.0}, {"spread": 30.0, "stagger": 4.0, "offset": {"x": 2.0, "y": 0.0}}
    ]);
    document["matrices"][0]["rowLayout"] = json!([
        {"shift": {"x": 3.0, "y": 0.0}, "bend": 90.0, "offset": {"x": 1.0, "y": 0.0}}
    ]);
    document["layouts"] = json!([{
        "id": "layout", "name": "Layout", "boardId": "board",
        "matrixIds": ["matrix"], "partIds": []
    }]);
    let mut engine = CoreEngine::new();
    let reply = open(&mut engine, "grouped", &document);
    assert_eq!(reply["kind"], "scene", "{reply}");
    let saved = snapshot(&mut engine);
    assert_eq!(saved["formatVersion"], 7);
    assert_eq!(
        saved["matrices"][0]["columnLayout"],
        document["matrices"][0]["columnLayout"]
    );
    assert_eq!(
        saved["matrices"][0]["rowLayout"],
        document["matrices"][0]["rowLayout"]
    );
    assert_eq!(saved["layouts"][0]["matrixIds"], json!(["matrix"]));
    let projected = &reply["scene"]["matrixScenes"][0]["cells"];
    let near = |value: &Value, x: f64, y: f64| {
        assert!((value["pose"]["at"]["x"].as_f64().unwrap() - x).abs() < 1e-9);
        assert!((value["pose"]["at"]["y"].as_f64().unwrap() - y).abs() < 1e-9);
    };
    near(&projected[0], -1.0, 3.0);
    near(&projected[1], 33.0, 7.0);
    assert_eq!(
        open(&mut CoreEngine::new(), "round-trip", &saved)["kind"],
        "scene"
    );
    let mut old = document;
    old["formatVersion"] = json!(6);
    assert_error(&open(&mut engine, "old", &old));
    assert_eq!(snapshot(&mut engine), saved);
}

#[test]
fn grouped_layout_validation_refuses_bad_lists_and_unresolved_anchors_atomically() {
    let mut document = baseline();
    document["constraints"] = json!([]);
    document["layouts"] = json!([{
        "id": "layout", "name": "Layout", "boardId": "board",
        "matrixIds": ["matrix"], "partIds": []
    }]);
    let mut engine = CoreEngine::new();
    assert_eq!(open(&mut engine, "valid", &document)["kind"], "scene");
    let accepted = snapshot(&mut engine);
    for (field, value) in [
        ("columnLayout", json!([{}, {}, {}])),
        ("rowLayout", json!([{}, {}])),
        ("columnLayout", json!([{"spread": 0.0}])),
        ("rowLayout", json!([{"padding": -2.0}])),
        ("anchor", json!({"matrixId": "missing", "row": 0, "column": 0})),
        ("anchor", json!({"matrixId": "matrix", "row": 0, "column": 0})),
    ] {
        let mut invalid = document.clone();
        invalid["matrices"][0][field] = value;
        assert_error(&open(&mut engine, "invalid-layout", &invalid));
        assert_eq!(snapshot(&mut engine), accepted);
    }
    let mut empty = document;
    empty["layouts"][0]["matrixIds"] = json!([]);
    assert_error(&open(&mut engine, "empty-layout", &empty));
    assert_eq!(snapshot(&mut engine), accepted);
}

#[test]
fn production_layout_keeps_invisible_carried_steps_and_anchor_frames() {
    for (enabled, deleted) in [(false, false), (true, true), (true, false)] {
        let mut document = baseline();
        document["constraints"] = json!([]);
        document["parts"] = json!([{
            "id": "matrix/matrix/r1c0", "definitionId": "def", "reference": "SW1",
            "pose": {"at": {"x": 0, "y": 0}, "rotation": 0}, "side": "front"
        }]);
        document["boards"][0]["partIds"] = json!(["matrix/matrix/r1c0"]);
        document["matrices"][0]["rows"] = json!(2);
        document["matrices"][0]["columns"] = json!(1);
        document["matrices"][0]["partIds"] = json!(["matrix/matrix/r1c0"]);
        document["matrices"][0]["cells"] = json!([{
            "row": 0, "column": 0, "enabled": enabled, "deleted": deleted,
            "shift": {"x": 2.0, "y": 0.0}, "bend": 90.0
        }]);
        document["matrices"].as_array_mut().unwrap().push(json!({
            "id": "child", "boardId": "board", "rows": 1, "columns": 1,
            "pitch": {"x": 19.0, "y": 19.0}, "origin": {"x": 0.0, "y": -19.0},
            "definitionId": "def", "partIds": [],
            "anchor": {"matrixId": "matrix", "row": 0, "column": 0}
        }));
        document["layouts"] = json!([{
            "id": "layout", "name": "Layout", "boardId": "board",
            "matrixIds": ["child", "matrix"], "partIds": []
        }]);
        let mut engine = CoreEngine::new();
        let reply = open(&mut engine, "carried", &document);
        assert_eq!(reply["kind"], "scene", "{reply}");
        let parent = &reply["scene"]["matrixScenes"][0];
        let later = parent["cells"].as_array().unwrap().iter().find(|cell| cell["row"] == 1).unwrap();
        assert!((later["pose"]["at"]["x"].as_f64().unwrap() + 17.0).abs() < 1e-9);
        assert!(later["pose"]["at"]["y"].as_f64().unwrap().abs() < 1e-9);
        assert_eq!(later["pose"]["rotation"], 90.0);
        assert_eq!(reply["document"]["parts"][0]["pose"], later["pose"]);
        assert!((later["offsetFrame"]["axisX"]["y"].as_f64().unwrap() - 1.0).abs() < 1e-9);
        let child = &reply["scene"]["matrixScenes"][1]["cells"][0];
        assert!((child["pose"]["at"]["x"].as_f64().unwrap() - 21.0).abs() < 1e-9);
        assert_eq!(child["pose"]["rotation"], 90.0);
        if !enabled || deleted {
            assert!(!reply["document"]["parts"].as_array().unwrap().iter().any(|part| part["id"] == "matrix/matrix/r0c0"));
        }
    }
}

#[test]
fn omitted_matrix_membership_keeps_a_live_anchor_without_materialized_parts() {
    let mut document = baseline();
    document["constraints"] = json!([]);
    document["parts"] = json!([]);
    document["boards"][0]["partIds"] = json!([]);
    document["matrices"][0]["columns"] = json!(1);
    document["matrices"][0]["partIds"] = json!([]);
    document["matrices"][0]["cells"] = json!([{
        "row": 0, "column": 0, "enabled": true,
        "shift": {"x": 2.0, "y": 0.0}, "bend": 90.0
    }]);
    let child = json!({
        "id": "child", "boardId": "board", "rows": 1, "columns": 1,
        "pitch": {"x": 19.0, "y": 19.0}, "origin": {"x": 0.0, "y": -19.0},
        "definitionId": "def", "partIds": [],
        "anchor": {"matrixId": "matrix", "row": 0, "column": 0}
    });
    let mut mirror_child = child.clone();
    mirror_child["id"] = json!("mirror-child");
    mirror_child["anchor"]["matrixId"] = json!("omitted");
    mirror_child["mirror"] = json!("x");
    document["matrices"].as_array_mut().unwrap().extend([mirror_child, child]);
    document["layouts"] = json!([
        {"id": "source", "name": "Source", "boardId": "board", "matrixIds": ["child", "matrix"], "partIds": []},
        {"id": "mirror", "name": "Mirror", "boardId": "board", "matrixIds": ["omitted", "mirror-child"], "partIds": [],
         "mirrorLink": {"sourceId": "source", "axisX": 50.0,
            "correspondences": [
                {"kind": "matrix", "sourceId": "child", "mirrorId": "mirror-child"},
                {"kind": "matrix", "sourceId": "matrix", "mirrorId": "omitted"},
                {"kind": "part", "sourceId": "matrix/matrix/r0c0", "mirrorId": "matrix/omitted/r0c0"}
            ],
            "exceptions": [{"kind": "matrix", "mirrorId": "omitted", "state": "omitted"}]
        }}
    ]);
    let mut engine = CoreEngine::new();
    let opened = open(&mut engine, "omitted", &document);
    assert_eq!(opened["kind"], "scene", "{opened}");
    let child_pose = |reply: &Value| reply["document"]["parts"].as_array().unwrap().iter().find(|part| part["id"] == "matrix/mirror-child/r0c0").unwrap()["pose"].clone();
    assert!((child_pose(&opened)["at"]["x"].as_f64().unwrap() - 79.0).abs() < 1e-9);
    assert_eq!(child_pose(&opened)["rotation"], -90.0);
    let mut changed = opened["document"]["matrices"][0].clone();
    changed["cells"][0]["shift"]["x"] = json!(5.0);
    let edited = request(&mut engine, json!({"kind":"edit","id":"move-source","command":{
        "baseRevision":0,"transactionId":"move-source","phase":"commit","targetIds":[],
        "operation":{"kind":"set-matrix","matrix":changed}
    }}));
    assert_eq!(edited["kind"], "scene", "{edited}");
    assert!((child_pose(&edited)["at"]["x"].as_f64().unwrap() - 76.0).abs() < 1e-9, "{edited}");
    assert!(!edited["document"]["parts"].as_array().unwrap().iter().any(|part| part["id"].as_str().unwrap().starts_with("matrix/omitted/")));
    assert_eq!(edited["document"]["layouts"][1]["matrixIds"], json!(["omitted", "mirror-child"]));
    let undone = request(&mut engine, json!({"kind":"undo","id":"undo"}));
    assert_eq!(child_pose(&undone), child_pose(&opened));
    let redone = request(&mut engine, json!({"kind":"redo","id":"redo"}));
    assert_eq!(child_pose(&redone), child_pose(&edited));
}

#[test]
fn incompatible_format_refusals_explain_recovery_and_preserve_history() {
    use boardstudio_core::migrate::CURRENT_VERSION;

    for route in ["open", "replace"] {
        let mut engine = CoreEngine::new();
        assert_eq!(open(&mut engine, "open", &baseline())["kind"], "scene");
        let committed = request(
            &mut engine,
            json!({"kind":"edit","id":"wiring","command":{
                "baseRevision":0,"transactionId":"wiring","phase":"commit","targetIds":[],
                "operation":{"kind":"set-wiring-mode","boardId":"board","mode":"direct"}
            }}),
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        let accepted = snapshot(&mut engine);
        for (version, explanation) in [
            (json!(CURRENT_VERSION - 1), "No conversion is available"),
            (json!(CURRENT_VERSION + 1), "compatible newer build"),
            (json!(0), "invalid formatVersion"),
            (json!("7"), "invalid formatVersion"),
        ] {
            let mut candidate = baseline();
            candidate["formatVersion"] = version;
            let reply = if route == "open" {
                open(&mut engine, "refused", &candidate)
            } else {
                request(
                    &mut engine,
                    json!({"kind":"edit","id":"refused","command":{
                        "baseRevision":accepted["revision"],"transactionId":"refused",
                        "phase":"commit","targetIds":[],
                        "operation":{"kind":"replace-document","document":candidate}
                    }}),
                )
            };
            assert_error(&reply);
            let message = reply["message"].as_str().unwrap();
            assert!(message.contains(explanation), "{route}: {message}");
            assert_eq!(snapshot(&mut engine), accepted, "{route} changed state");
        }
        assert_eq!(
            request(&mut engine, json!({"kind":"undo","id":"undo"}))["kind"],
            "scene",
            "{route} cleared Undo"
        );
        let redone = request(&mut engine, json!({"kind":"redo","id":"redo"}));
        assert_eq!(redone["kind"], "scene", "{route} cleared Redo");
        let mut restored = redone["document"].clone();
        restored["revision"] = accepted["revision"].clone();
        assert_eq!(restored, accepted, "{route} changed history contents");
    }
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
