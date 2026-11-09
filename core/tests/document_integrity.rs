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

/// A refusal for the stated reason, not merely any refusal.
fn assert_refused(reply: &Value, reason: &str) {
    assert_error(reply);
    let message = reply["message"].as_str().unwrap_or_default();
    assert!(
        message.contains(reason),
        "expected a refusal containing {reason:?}, got {message:?}"
    );
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
    for (field, value, reason) in [
        (
            "columnLayout",
            json!([{}, {}, {}]),
            "layout list exceeds axis count",
        ),
        (
            "rowLayout",
            json!([{}, {}]),
            "layout list exceeds axis count",
        ),
        (
            "columnLayout",
            json!([{"spread": 0.0}]),
            "no Spread or Padding at axis zero",
        ),
        (
            "rowLayout",
            json!([{"padding": -2.0}]),
            "no Spread or Padding at axis zero",
        ),
        (
            "anchor",
            json!({"matrixId": "missing", "row": 0, "column": 0}),
            "anchor target missing is missing",
        ),
        (
            "anchor",
            json!({"matrixId": "matrix", "row": 0, "column": 0}),
            "Matrix anchor cycle at matrix",
        ),
    ] {
        let mut invalid = document.clone();
        invalid["matrices"][0][field] = value;
        assert_refused(&open(&mut engine, "invalid-layout", &invalid), reason);
        assert_eq!(snapshot(&mut engine), accepted);
    }
    let mut empty = document;
    empty["layouts"][0]["matrixIds"] = json!([]);
    assert_refused(
        &open(&mut engine, "empty-layout", &empty),
        "Layout matrix membership must be nonempty",
    );
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
        let later = parent["cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|cell| cell["row"] == 1)
            .unwrap();
        assert!((later["pose"]["at"]["x"].as_f64().unwrap() + 17.0).abs() < 1e-9);
        assert!(later["pose"]["at"]["y"].as_f64().unwrap().abs() < 1e-9);
        assert_eq!(later["pose"]["rotation"], 90.0);
        assert_eq!(reply["document"]["parts"][0]["pose"], later["pose"]);
        assert!((later["offsetFrame"]["axisX"]["y"].as_f64().unwrap() - 1.0).abs() < 1e-9);
        let child = &reply["scene"]["matrixScenes"][1]["cells"][0];
        assert!((child["pose"]["at"]["x"].as_f64().unwrap() - 21.0).abs() < 1e-9);
        assert_eq!(child["pose"]["rotation"], 90.0);
        if !enabled || deleted {
            assert!(
                !reply["document"]["parts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|part| part["id"] == "matrix/matrix/r0c0")
            );
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
    document["matrices"]
        .as_array_mut()
        .unwrap()
        .extend([mirror_child, child]);
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
    let mut invalid = opened["document"].clone();
    let records = invalid["layouts"][1]["mirrorLink"]["correspondences"]
        .as_array_mut()
        .unwrap();
    records.push(records[0].clone());
    assert_error(&open(&mut engine, "duplicate-correspondence", &invalid));
    assert_eq!(snapshot(&mut engine), opened["document"]);
    let child_pose = |reply: &Value| {
        reply["document"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|part| part["id"] == "matrix/mirror-child/r0c0")
            .unwrap()["pose"]
            .clone()
    };
    assert!((child_pose(&opened)["at"]["x"].as_f64().unwrap() - 79.0).abs() < 1e-9);
    assert_eq!(child_pose(&opened)["rotation"], -90.0);
    let mut changed = opened["document"]["matrices"][0].clone();
    changed["cells"][0]["shift"]["x"] = json!(5.0);
    let edited = request(
        &mut engine,
        json!({"kind":"edit","id":"move-source","command":{
            "baseRevision":0,"transactionId":"move-source","phase":"commit","targetIds":[],
            "operation":{"kind":"set-matrix","matrix":changed}
        }}),
    );
    assert_eq!(edited["kind"], "scene", "{edited}");
    assert!(
        (child_pose(&edited)["at"]["x"].as_f64().unwrap() - 76.0).abs() < 1e-9,
        "{edited}"
    );
    assert!(
        !edited["document"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["id"].as_str().unwrap().starts_with("matrix/omitted/"))
    );
    assert_eq!(
        edited["document"]["layouts"][1]["matrixIds"],
        json!(["omitted", "mirror-child"])
    );
    let undone = request(&mut engine, json!({"kind":"undo","id":"undo"}));
    assert_eq!(child_pose(&undone), child_pose(&opened));
    let redone = request(&mut engine, json!({"kind":"redo","id":"redo"}));
    assert_eq!(child_pose(&redone), child_pose(&edited));
}

#[test]
fn rigid_column_translation_uses_steps_under_bend_and_preserves_other_columns() {
    let mut document = baseline();
    document["constraints"] = json!([]);
    let mut engine = CoreEngine::new();
    assert_eq!(open(&mut engine, "start", &document)["kind"], "scene");
    let mut matrix = document["matrices"][0].clone();
    matrix["rows"] = json!(2);
    matrix["columns"] = json!(3);
    matrix["columnLayout"] = json!([{"splay": 12.0}, {"rotation": 18.0}]);
    matrix["rowLayout"] = json!([{"bend": 30.0}]);
    let mut accepted = request(
        &mut engine,
        json!({"kind":"edit","id":"setup","command":{
            "baseRevision":0,"transactionId":"setup","phase":"commit","targetIds":[],
            "operation":{"kind":"set-matrix","matrix":matrix}
        }}),
    );
    assert_eq!(accepted["kind"], "scene", "{accepted}");
    for column in 0..3 {
        let edited = request(
            &mut engine,
            json!({"kind":"edit","id":"align","command":{
                "baseRevision":accepted["document"]["revision"],"transactionId":"align","phase":"commit","targetIds":[],
                "operation":{"kind":"set-matrix-placement","matrixId":"matrix", "change":{
                    "kind":"column-translation", "column":column, "delta":{"x":3.0,"y":4.0}
                }}
            }}),
        );
        assert_eq!(edited["kind"], "scene", "{edited}");
        for row in 0..2 {
            for other in 0..3 {
                let id = format!("matrix/matrix/r{row}c{other}");
                let pose = |reply: &Value| {
                    reply["document"]["parts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|part| part["id"] == id)
                        .unwrap()["pose"]
                        .clone()
                };
                let before = pose(&accepted);
                let after = pose(&edited);
                for (axis, amount) in [("x", 3.0), ("y", 4.0)] {
                    let expected = before["at"][axis].as_f64().unwrap()
                        + if other == column { amount } else { 0.0 };
                    assert!((after["at"][axis].as_f64().unwrap() - expected).abs() < 1e-9);
                }
                assert_eq!(after["rotation"], before["rotation"]);
            }
        }
        accepted = edited;
    }
}

#[test]
fn explicit_steps_survive_pitch_edits_and_compaction_drops_only_removed_records() {
    let mut document = baseline();
    document["constraints"] = json!([]);
    document["parameters"]["unrelated"] = json!({"keep": [1, 2, 3]});
    let mut engine = CoreEngine::new();
    assert_eq!(open(&mut engine, "steps", &document)["kind"], "scene");
    let mut matrix = document["matrices"][0].clone();
    matrix["rows"] = json!(4);
    matrix["columns"] = json!(4);
    matrix["columnLayout"] = json!([{}, {"spread": -5.0, "stagger": 2.0}, {"spread": 30.0}]);
    matrix["rowLayout"] = json!([{}, {"padding": 0.0}, {"padding": -4.0}]);
    let set = |engine: &mut CoreEngine, revision: u64, matrix: Value| {
        let reply = request(
            engine,
            json!({"kind":"edit","id":"steps","command":{
                "baseRevision":revision,"transactionId":"steps","phase":"commit","targetIds":[],
                "operation":{"kind":"set-matrix","matrix":matrix}
            }}),
        );
        assert_eq!(reply["kind"], "scene", "{reply}");
        reply
    };
    let initial = set(&mut engine, 0, matrix);
    let mut matrix = initial["document"]["matrices"][0].clone();
    matrix["pitch"] = json!({"x":40.0,"y":50.0});
    let pitched = set(&mut engine, 1, matrix);
    let key = |reply: &Value, row: u32, column: u32| {
        reply["scene"]["matrixScenes"][0]["cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|cell| cell["row"] == row && cell["column"] == column)
            .unwrap()["pose"]["at"]
            .clone()
    };
    assert_eq!(key(&pitched, 2, 2), json!({"x":25.0,"y":-2.0}));
    assert_eq!(key(&pitched, 3, 3), json!({"x":65.0,"y":48.0}));
    let mut matrix = pitched["document"]["matrices"][0].clone();
    matrix["rows"] = json!(2);
    matrix["columns"] = json!(2);
    let shrunk = set(&mut engine, 2, matrix);
    assert_eq!(
        shrunk["document"]["matrices"][0]["columnLayout"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        shrunk["document"]["matrices"][0]["rowLayout"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let mut matrix = shrunk["document"]["matrices"][0].clone();
    matrix["rows"] = json!(4);
    matrix["columns"] = json!(4);
    let grown = set(&mut engine, 3, matrix);
    assert_eq!(key(&grown, 2, 2), json!({"x":35.0,"y":52.0}));
    let compacted = request(
        &mut engine,
        json!({"kind":"edit","id":"remove-first-column","command":{
            "baseRevision":4,"transactionId":"remove-first-column","phase":"commit","targetIds":[],
            "operation":{"kind":"remove-matrix-column","matrixId":"matrix","column":0}
        }}),
    );
    assert_eq!(compacted["kind"], "scene", "{compacted}");
    assert!(
        compacted["document"]["matrices"][0]["columnLayout"][0]
            .get("spread")
            .is_none()
    );
    assert_eq!(key(&compacted, 0, 0), json!({"x":0.0,"y":2.0}));
    assert_eq!(
        compacted["document"]["parameters"]["unrelated"],
        document["parameters"]["unrelated"]
    );
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
fn malformed_missing_and_foreign_format_inputs_refuse_before_any_state_change() {
    for route in ["open", "replace"] {
        let mut engine = CoreEngine::new();
        assert_eq!(open(&mut engine, "open", &baseline())["kind"], "scene");
        let accepted = snapshot(&mut engine);
        let mut cases: Vec<(Value, &str)> = Vec::new();
        let mut missing = baseline();
        missing.as_object_mut().unwrap().remove("formatVersion");
        cases.push((missing, "no formatVersion"));
        for version in [json!(null), json!(7.5), json!(-1), json!(true), json!([7])] {
            let mut candidate = baseline();
            candidate["formatVersion"] = version;
            cases.push((candidate, "invalid formatVersion"));
        }
        let mut foreign = baseline();
        foreign["format"] = json!("boardstudio/v1");
        foreign["formatVersion"] = json!(6);
        cases.push((foreign, "Unsupported"));
        cases.push((json!([]), "must be an object"));
        for (candidate, explanation) in cases {
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
            assert!(
                message.contains(explanation),
                "{route} {candidate}: {message}"
            );
            assert_eq!(snapshot(&mut engine), accepted, "{route} changed state");
        }
    }
}

#[test]
fn malformed_json_text_is_refused_without_changing_accepted_state() {
    let mut engine = CoreEngine::new();
    assert_eq!(open(&mut engine, "open", &baseline())["kind"], "scene");
    let accepted = snapshot(&mut engine);
    for text in ["{not json", "{\"kind\":\"open\",\"document\":{\"format\":"] {
        let reply: Value = serde_json::from_str(&engine.request(text)).unwrap();
        assert_error(&reply);
        assert_eq!(snapshot(&mut engine), accepted, "{text} changed state");
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

fn two_matrix_document() -> Value {
    let mut document = baseline();
    document["constraints"] = json!([]);
    document["parts"].as_array_mut().unwrap().push(json!(
        {"id":"c","definitionId":"def","reference":"SW3","pose":{"at":{"x":0,"y":-19},"rotation":0},"side":"front"}
    ));
    document["boards"][0]["partIds"] = json!(["a", "b", "c"]);
    document["matrices"].as_array_mut().unwrap().push(json!({
        "id":"second","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":-19},
        "definitionId":"def","partIds":["c"],"boardId":"board"
    }));
    document["layouts"] = json!([{
        "id": "layout", "name": "Layout", "boardId": "board",
        "matrixIds": ["matrix", "second"], "partIds": []
    }]);
    document
}

#[test]
fn zero_and_negative_steps_are_accepted_while_axis_zero_steps_are_refused() {
    let mut engine = CoreEngine::new();
    let mut valid = two_matrix_document();
    valid["matrices"][0]["columnLayout"] = json!([{}, {"spread": -4.0}]);
    valid["matrices"][0]["rowLayout"] = json!([]);
    assert_eq!(open(&mut engine, "negative", &valid)["kind"], "scene");
    valid["matrices"][0]["columnLayout"] = json!([{}, {"spread": 0.0}]);
    assert_eq!(open(&mut engine, "zero", &valid)["kind"], "scene");
    let accepted = snapshot(&mut engine);
    let mut axis_zero = valid.clone();
    axis_zero["matrices"][0]["columnLayout"] = json!([{"spread": 3.0}]);
    assert_refused(
        &open(&mut engine, "axis-zero", &axis_zero),
        "no Spread or Padding at axis zero",
    );
    assert_eq!(snapshot(&mut engine), accepted);
}

#[test]
fn anchors_must_resolve_to_an_in_range_key_in_the_same_layout() {
    let mut engine = CoreEngine::new();
    let mut valid = two_matrix_document();
    valid["matrices"][1]["anchor"] = json!({"matrixId": "matrix", "row": 0, "column": 1});
    assert_eq!(open(&mut engine, "valid", &valid)["kind"], "scene");
    let accepted = snapshot(&mut engine);
    let mut cases = Vec::new();
    let mut out_of_range = valid.clone();
    out_of_range["matrices"][1]["anchor"]["column"] = json!(2);
    cases.push((
        "out-of-range",
        out_of_range,
        "anchor key matrix/matrix/r0c2 is missing",
    ));
    let mut other_layout = valid.clone();
    other_layout["layouts"] = json!([
        {"id":"layout","name":"Layout","boardId":"board","matrixIds":["matrix"],"partIds":[]},
        {"id":"other","name":"Other","boardId":"board","matrixIds":["second"],"partIds":[]}
    ]);
    cases.push((
        "other-layout",
        other_layout,
        "anchor must stay on the same board and Layout",
    ));
    let mut unowned = valid.clone();
    unowned["layouts"][0]["matrixIds"] = json!(["matrix"]);
    cases.push((
        "unowned",
        unowned,
        "Anchored matrix second must belong to a Layout",
    ));
    let mut cycle = valid.clone();
    cycle["matrices"][0]["anchor"] = json!({"matrixId": "second", "row": 0, "column": 0});
    cases.push(("cycle", cycle, "Matrix anchor cycle at"));
    let mut shared = valid.clone();
    shared["layouts"].as_array_mut().unwrap().push(json!(
        {"id":"again","name":"Again","boardId":"board","matrixIds":["second"],"partIds":[]}
    ));
    cases.push((
        "two-layouts",
        shared,
        "Each layout must own distinct matrices on its board",
    ));
    for (name, case, reason) in cases {
        assert_refused(&open(&mut engine, name, &case), reason);
        assert_eq!(snapshot(&mut engine), accepted, "{name}");
    }
}

#[test]
fn anchors_without_any_layout_are_refused_not_read_as_world_origin() {
    let mut engine = CoreEngine::new();
    let mut document = two_matrix_document();
    document["matrices"][1]["anchor"] = json!({"matrixId": "matrix", "row": 0, "column": 0});
    document["layouts"] = json!([]);
    assert_refused(
        &open(&mut engine, "no-layout", &document),
        "must belong to a Layout",
    );
}

#[test]
fn non_finite_evaluated_layouts_and_empty_ownership_are_refused() {
    let mut engine = CoreEngine::new();
    let valid = two_matrix_document();
    assert_eq!(open(&mut engine, "valid", &valid)["kind"], "scene");
    let accepted = snapshot(&mut engine);
    let mut overflow = valid.clone();
    overflow["matrices"][0]["columnLayout"] =
        json!([{}, {"offset": {"x": 1e308, "y": 0.0}, "stagger": 1e308}]);
    overflow["matrices"][0]["cells"] =
        json!([{"row":0,"column":1,"enabled":true,"shift":{"x":1.7e308,"y":0.0}}]);
    let mut orphan = valid.clone();
    orphan["layouts"][0]["matrixIds"] = json!(["matrix", "missing"]);
    let mut duplicate_member = valid.clone();
    duplicate_member["layouts"][0]["matrixIds"] = json!(["matrix", "matrix"]);
    let mut other_board = valid.clone();
    other_board["boards"].as_array_mut().unwrap().push(json!({
        "id":"board2","name":"Board 2","outlineIds":[],"partIds":[],"netIds":[],"thickness":1.6
    }));
    other_board["matrices"][1]["boardId"] = json!("board2");
    other_board["boards"][0]["partIds"] = json!(["a", "b"]);
    other_board["boards"][1]["partIds"] = json!(["c"]);
    for (name, case, reason) in [
        (
            "overflow",
            overflow,
            "layout values and evaluated results must be finite",
        ),
        ("orphan", orphan, "Logical matrix missing is missing"),
        (
            "duplicate-member",
            duplicate_member,
            "Each layout must own distinct matrices on its board",
        ),
        (
            "other-board",
            other_board,
            "Each layout must own distinct matrices on its board",
        ),
    ] {
        assert_refused(&open(&mut engine, name, &case), reason);
        assert_eq!(snapshot(&mut engine), accepted, "{name}");
    }
}

fn edit(engine: &mut CoreEngine, revision: u64, operation: Value) -> Value {
    request(
        engine,
        json!({"kind":"edit","id":"probe","command":{
            "baseRevision":revision,"transactionId":"probe","phase":"commit","targetIds":[],
            "operation":operation
        }}),
    )
}

#[test]
fn edits_that_orphan_an_anchor_or_empty_a_layout_are_refused_atomically() {
    let mut engine = CoreEngine::new();
    let mut document = two_matrix_document();
    document["matrices"][1]["anchor"] = json!({"matrixId": "matrix", "row": 0, "column": 1});
    let opened = open(&mut engine, "valid", &document);
    assert_eq!(opened["kind"], "scene");
    let accepted = snapshot(&mut engine);
    let laid_out = opened["scene"]["matrixScenes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|scene| scene["matrixId"] == "second")
        .expect("anchored scene")["cells"][0]["pose"]
        .clone();
    let mut shrunk = accepted["matrices"][0].clone();
    shrunk["columns"] = json!(1);
    shrunk["partIds"] = json!(["a"]);
    // Ticket 19: shrinking detaches dependants with their frame instead of
    // refusing. The anchor never retargets the new key at the old coordinate.
    let reply = edit(&mut engine, 0, json!({"kind":"set-matrix","matrix":shrunk}));
    assert_eq!(reply["kind"], "scene", "{reply}");
    let detached = snapshot(&mut engine);
    assert_eq!(detached["matrices"][1]["anchor"], json!(null));
    assert_eq!(
        detached["matrices"][1]["origin"],
        json!({"x": 19.0, "y": -19.0})
    );
    let member_c = detached["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["id"] == "matrix/second/r0c0")
        .expect("detached member is kept");
    // The stale authored id "c" is normalized to its member id; its pose is
    // the laid-out one, unchanged by the detach.
    assert_eq!(member_c["pose"], laid_out);
    let mut unowned = detached["matrices"][1].clone();
    unowned["anchor"] = json!({"matrixId": "missing", "row": 0, "column": 0});
    assert_refused(
        &edit(
            &mut engine,
            1,
            json!({"kind":"set-matrix","matrix":unowned}),
        ),
        "anchor target missing is missing",
    );
    assert_eq!(snapshot(&mut engine), detached);
}

#[test]
fn an_anchor_without_follow_follows_all_three_components() {
    let pose = |follow: Option<Value>| {
        let mut engine = CoreEngine::new();
        let mut document = two_matrix_document();
        document["matrices"][0]["rotation"] = json!(30.0);
        document["matrices"][1]["anchor"] = json!({"matrixId": "matrix", "row": 0, "column": 1});
        if let Some(follow) = follow {
            document["matrices"][1]["anchor"]["follow"] = follow;
        }
        let reply = open(&mut engine, "follow", &document);
        assert_eq!(reply["kind"], "scene", "{reply}");
        reply["scene"]["matrixScenes"][1]["cells"][0]["pose"].clone()
    };
    let default = pose(None);
    assert_eq!(default, pose(Some(json!(["x", "y", "rotation"]))));
    let own = pose(Some(json!([])));
    assert_ne!(default, own);
    // Following nothing contributes no anchor translation or rotation (pose pinned from output).
    let number = |value: &Value| value.as_f64().unwrap();
    assert_eq!(number(&own["at"]["x"]), 0.0);
    assert_eq!(number(&own["at"]["y"]), 0.0);
    assert_eq!(number(&own["rotation"]), 0.0);
}

/// Renames every string equal to `old` anywhere in the document.
fn rename_id(value: &mut Value, old: &str, new: &str) {
    match value {
        Value::String(text) if text == old => *text = new.to_string(),
        Value::Array(items) => items.iter_mut().for_each(|item| rename_id(item, old, new)),
        Value::Object(fields) => fields
            .values_mut()
            .for_each(|item| rename_id(item, old, new)),
        _ => {}
    }
}

#[test]
fn anchor_keys_stay_free_of_constraints_and_self_anchors_are_refused() {
    let mut engine = CoreEngine::new();
    let mut document = two_matrix_document();
    document["matrices"][1]["anchor"] = json!({"matrixId": "matrix", "row": 0, "column": 1});
    assert_eq!(open(&mut engine, "valid", &document)["kind"], "scene");
    let accepted = snapshot(&mut engine);
    let mut constrained = document.clone();
    for (old, new) in [
        ("a", "matrix/matrix/r0c0"),
        ("b", "matrix/matrix/r0c1"),
        ("c", "matrix/second/r0c0"),
    ] {
        rename_id(&mut constrained, old, new);
    }
    constrained["constraints"] = json!([{
        "kind":"offset","id":"constraint","sourcePartId":"matrix/matrix/r0c0",
        "targetPartId":"matrix/matrix/r0c1","offset":{"x":19,"y":0},"rotation":0
    }]);
    let mut itself = document.clone();
    itself["matrices"][1]["anchor"] = json!({"matrixId": "second", "row": 0, "column": 0});
    let mut part_in_matrix_layout = document.clone();
    part_in_matrix_layout["layouts"][0]["partIds"] = json!(["a"]);
    let mut duplicate_parts = document.clone();
    duplicate_parts["parts"].as_array_mut().unwrap().push(json!(
        {"id":"d","definitionId":"def","reference":"SW4","pose":{"at":{"x":0,"y":40},"rotation":0},"side":"front"}
    ));
    duplicate_parts["boards"][0]["partIds"] = json!(["a", "b", "c", "d"]);
    duplicate_parts["layouts"][0]["partIds"] = json!(["d", "d"]);
    for (name, case, reason) in [
        (
            "constrained-anchor-key",
            constrained,
            "is constraint-driven",
        ),
        ("self-anchor", itself, "Matrix anchor cycle at second"),
        (
            "matrix-part-in-layout",
            part_in_matrix_layout,
            "Layout components must be independent parts on the same board",
        ),
        (
            "duplicate-parts",
            duplicate_parts,
            "Layout components must be independent parts on the same board",
        ),
    ] {
        assert_refused(&open(&mut engine, name, &case), reason);
        assert_eq!(snapshot(&mut engine), accepted, "{name}");
    }
}

const DIMENSION_REFUSAL: &str = "Matrix dimensions are invalid or exceed the part limit";
// MAX_MATRIX_PARTS (core/src/matrix.rs) + 1; the constant is pub(crate), so tests cannot import it.
const ONE_OVER_PART_LIMIT: u32 = 4097;

fn anchored_document() -> Value {
    let mut document = two_matrix_document();
    document["matrices"][1]["anchor"] = json!({"matrixId": "matrix", "row": 0, "column": 1});
    document
}

fn with_dimensions(document: &Value, rows: u32, columns: u32) -> Value {
    let mut case = document.clone();
    case["matrices"][0]["rows"] = json!(rows);
    case["matrices"][0]["columns"] = json!(columns);
    case
}

#[test]
fn a_matrix_of_u32_max_rows_is_refused_on_open_before_any_row_walk() {
    let mut engine = CoreEngine::new();
    let document = anchored_document();
    assert_eq!(open(&mut engine, "valid", &document)["kind"], "scene");
    let accepted = snapshot(&mut engine);
    let huge = with_dimensions(&document, u32::MAX, 2);
    assert_refused(&open(&mut engine, "huge", &huge), DIMENSION_REFUSAL);
    assert_eq!(snapshot(&mut engine), accepted);
}

#[test]
fn a_matrix_of_u32_max_columns_is_refused_on_open_before_any_column_walk() {
    let mut engine = CoreEngine::new();
    let document = anchored_document();
    assert_eq!(open(&mut engine, "valid", &document)["kind"], "scene");
    let accepted = snapshot(&mut engine);
    let huge = with_dimensions(&document, 1, u32::MAX);
    assert_refused(&open(&mut engine, "huge", &huge), DIMENSION_REFUSAL);
    assert_eq!(snapshot(&mut engine), accepted);
}

#[test]
fn a_matrix_one_over_the_part_limit_is_refused_on_open() {
    let mut engine = CoreEngine::new();
    let document = anchored_document();
    assert_eq!(open(&mut engine, "valid", &document)["kind"], "scene");
    let accepted = snapshot(&mut engine);
    let over = with_dimensions(&document, ONE_OVER_PART_LIMIT, 1);
    assert_refused(&open(&mut engine, "over", &over), DIMENSION_REFUSAL);
    assert_eq!(snapshot(&mut engine), accepted);
}

#[test]
fn oversized_matrices_are_refused_through_set_matrix_and_project_matrices_with_state_unchanged() {
    let mut engine = CoreEngine::new();
    let document = anchored_document();
    assert_eq!(open(&mut engine, "valid", &document)["kind"], "scene");
    let accepted = snapshot(&mut engine);
    for (rows, columns) in [(u32::MAX, 2), (1, u32::MAX), (ONE_OVER_PART_LIMIT, 1)] {
        let mut matrix = accepted["matrices"][0].clone();
        matrix["rows"] = json!(rows);
        matrix["columns"] = json!(columns);
        let reply = edit(
            &mut engine,
            0,
            json!({"kind":"set-matrix","matrix":matrix.clone()}),
        );
        assert_refused(&reply, DIMENSION_REFUSAL);
        assert_eq!(
            snapshot(&mut engine),
            accepted,
            "set-matrix {rows}x{columns}"
        );
        let reply = request(
            &mut engine,
            json!({"kind":"project-matrices","id":"project","baseRevision":0,"matrices":[matrix]}),
        );
        // The golden transcript pins the kernel's wording for draft projections.
        assert_refused(&reply, "invalid matrix dimensions");
        assert_eq!(snapshot(&mut engine), accepted, "project {rows}x{columns}");
    }
}

/// Characterization: cells beyond rows x columns are refused by `valid_matrix`.
#[test]
fn cells_beyond_the_matrix_dimensions_are_refused_by_set_matrix() {
    let mut engine = CoreEngine::new();
    let document = anchored_document();
    assert_eq!(open(&mut engine, "valid", &document)["kind"], "scene");
    let accepted = snapshot(&mut engine);
    let mut matrix = accepted["matrices"][1].clone();
    matrix["cells"] = json!([
        {"row":0,"column":0,"enabled":true},
        {"row":0,"column":1,"enabled":true}
    ]);
    let reply = edit(&mut engine, 0, json!({"kind":"set-matrix","matrix":matrix}));
    assert_refused(&reply, "Matrix cells exceed dimensions");
    assert_eq!(snapshot(&mut engine), accepted);
}
