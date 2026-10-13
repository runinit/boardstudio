//! `CreateMirroredHalf` (ticket 18) through Core's request surface using a small authored input.
#[path = "mirrored_half/activation_edges.rs"]
mod activation_edges;

use crate::support::synthetic_half;
use boardstudio_core::CoreEngine;
use boardstudio_core::mechanical::MechanicalTarget;
use boardstudio_core::mechanical::inputs::effective_inputs;
use boardstudio_core::model::ProjectDoc;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn request(engine: &mut CoreEngine, input: Value) -> Value {
    let reply: Value = serde_json::from_str(&engine.request(&input.to_string())).unwrap();
    assert_ne!(reply["kind"], "error", "{reply}");
    reply
}

/// Opens `document` in a fresh engine and returns the engine with the canonical document.
fn open(document: &Value) -> (CoreEngine, Value) {
    let mut engine = CoreEngine::new();
    let reply = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":document}),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    let document = reply["document"].clone();
    (engine, document)
}

fn snapshot(engine: &mut CoreEngine) -> Value {
    request(engine, json!({"id":"snapshot","kind":"snapshot"}))["document"].clone()
}

fn part<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
        .unwrap_or_else(|| panic!("part {id} is unavailable"))
}

fn angles_match(a: f64, b: f64) -> bool {
    let turn = (a - b).rem_euclid(360.0);
    turn.min(360.0 - turn) < 1e-9
}

fn mapping(axis: f64) -> Value {
    json!({"at": {"x": axis, "y": 0.0}, "rotation": 0.0})
}

fn new_board(board: &str, instance: &str) -> Value {
    json!({"kind":"new-board","boardId":board,"instanceId":instance})
}

/// The normalized bundled mounting-hole catalogue entry that closure projection needs.
fn mounting_hole() -> Value {
    json!({
        "id": "generator:ceoloide/mounting_hole_npth", "name": "mounting hole npth",
        "kind": "custom",
        "pads": [{ "id": "pad-0", "number": "", "at": { "x": 0, "y": 0 },
            "size": { "x": 2.2, "y": 2.2 }, "shape": "circle", "drill": 2.2, "plated": false, "rotation": 0 }],
        "courtyard": [{ "x": -1.1, "y": -1.1 }, { "x": 1.1, "y": -1.1 }, { "x": 1.1, "y": 1.1 }, { "x": -1.1, "y": 1.1 }],
        "envelopeSource": { "courtyard": "generated" },
        "envelopeNotice": "No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator": { "source": "ceoloide/mounting_hole_npth", "version": "bundled-1", "parameters": {} }
    })
}

/// An independently specified vertical reflection used by the synthetic contract tests.
fn synthetic_mapping(_document: &Value) -> Value {
    mapping(200.0)
}

/// Return the small authored half used by these shared operation tests.
fn seed_document(source: &Value, _board: &str) -> Value {
    source.clone()
}

fn seed(synthetic_input: &Value, board: &str) -> (CoreEngine, Value) {
    open(&seed_document(synthetic_input, board))
}

/// Supported prepared-route fixture: cell defaults inherit the matrix definition, and companion
/// assignments use the linked representation understood by the current reconciliation kernel.
fn runtime_supported_document() -> Value {
    let mut document = synthetic_half::document();
    for cell in document["matrices"][0]["cells"].as_array_mut().unwrap() {
        let object = cell.as_object_mut().unwrap();
        object.remove("definitionId");
        object.remove("variant");
        object.remove("assembliesLocal");
    }
    document
}

/// Submits one `CreateMirroredHalf` commit with the mounting-hole definition, and returns the
/// reply or the refusal message.
fn create(
    engine: &mut CoreEngine,
    document: &Value,
    pair: &str,
    source: &str,
    mode: Value,
    mapping: Value,
) -> Result<Value, String> {
    create_with(
        engine,
        document,
        "commit",
        operation(pair, source, mode, mapping),
    )
}

fn operation(pair: &str, source: &str, mode: Value, mapping: Value) -> Value {
    json!({
        "kind": "create-mirrored-half",
        "pairId": pair,
        "sourceInstanceId": source,
        "mode": mode,
        "mapping": mapping,
        "idRule": "board-prefix",
        "mountingHole": mounting_hole(),
    })
}

fn create_with(
    engine: &mut CoreEngine,
    document: &Value,
    phase: &str,
    operation: Value,
) -> Result<Value, String> {
    let reply: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"half","kind":"edit","command":{
            "baseRevision":document["revision"],"transactionId":"half","phase":phase,
            "targetIds":[],"operation":operation}})
            .to_string(),
        ),
    )
    .unwrap();
    if reply["kind"] == "error" {
        return Err(reply["message"].as_str().unwrap().to_owned());
    }
    Ok(reply)
}

fn pair<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["hardware"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
        .unwrap()
}

fn instance<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["hardware"]["instances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == id)
        .unwrap()
}

fn closure_holes(document: &Value, board: &str) -> BTreeSet<String> {
    document["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_owned())
        .filter(|id| id.starts_with(&format!("case-closure/{board}/")))
        .collect()
}

/// Checks the pair operation's position and heading rules for every authored correspondence.
fn assert_reflects_every_part(document: &Value, pair_id: &str, label: &str) {
    let record = pair(document, pair_id);
    let map = &record["mapping"];
    let (at_x, at_y, turn) = (
        map["at"]["x"].as_f64().unwrap(),
        map["at"]["y"].as_f64().unwrap(),
        map["rotation"].as_f64().unwrap(),
    );
    let (sin, cos) = turn.to_radians().sin_cos();
    let mut categories = (0, 0, 0);
    for correspondence in record["halves"]["correspondences"].as_array().unwrap() {
        if correspondence["kind"] != "part" {
            continue;
        }
        let source = part(document, correspondence["sourceId"].as_str().unwrap());
        let mirror = part(document, correspondence["mirrorId"].as_str().unwrap());
        let (x, y) = (
            source["pose"]["at"]["x"].as_f64().unwrap(),
            source["pose"]["at"]["y"].as_f64().unwrap(),
        );
        let expected = (-x * cos - y * sin + at_x, -x * sin + y * cos + at_y);
        let actual = (
            mirror["pose"]["at"]["x"].as_f64().unwrap(),
            mirror["pose"]["at"]["y"].as_f64().unwrap(),
        );
        let id = correspondence["sourceId"].as_str().unwrap();
        assert!(
            (actual.0 - expected.0).abs() < 1e-9 && (actual.1 - expected.1).abs() < 1e-9,
            "{label} {id}: at {actual:?}, expected {expected:?}"
        );
        let in_matrix = document["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|matrix| {
                matrix["partIds"].as_array().unwrap().iter().any(|member| {
                    member
                        .as_str()
                        .is_some_and(|member| id == member || id.starts_with(&format!("{member}/")))
                })
            });
        let source_heading = source["pose"]["rotation"].as_f64().unwrap();
        let expected_heading = if in_matrix {
            turn - source_heading
        } else {
            turn + 180.0 - source_heading
        };
        assert!(
            angles_match(
                mirror["pose"]["rotation"].as_f64().unwrap(),
                expected_heading
            ),
            "{label} {id}: heading does not follow the pair rule"
        );
        assert_eq!(mirror["side"], source["side"], "{label} {id}: side");
        if id.ends_with("/diode") || id.ends_with("/led") {
            categories.1 += 1;
        } else if id.starts_with("matrix/") {
            categories.0 += 1;
        } else {
            categories.2 += 1;
        }
    }
    assert_eq!(categories, (4, 8, 3), "{label}: authored part categories");
}

/// The effective mechanical configuration of one instance, as the Case workspace reads it.
fn effective(document: &Value, instance_id: &str) -> Value {
    let typed: ProjectDoc = serde_json::from_value(document.clone()).unwrap();
    serde_json::to_value(
        effective_inputs(
            &typed,
            &[],
            &MechanicalTarget::Instance {
                instance_id: instance_id.into(),
            },
        )
        .unwrap()
        .configuration,
    )
    .unwrap()
}

#[test]
fn a_rotated_pair_map_reflects_keys_companions_and_other_parts_by_the_pair_rule() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-rotated",
        "left",
        new_board("mirror", "mirror"),
        json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0}),
    )
    .unwrap();
    assert_reflects_every_part(&reply["document"], "pair-rotated", "synthetic");
}

#[test]
fn a_vertical_axis_map_reflects_every_part_by_the_pair_rule() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-flat",
        "left",
        new_board("mirror", "mirror"),
        mapping(300.0),
    )
    .unwrap();
    assert_reflects_every_part(&reply["document"], "pair-flat", "synthetic");
}

#[test]
fn creation_derives_independent_part_poses_for_axis_and_rotated_maps() {
    let synthetic_input = synthetic_half::document();
    for (pair_id, map) in [
        (
            "pair-axis",
            json!({"at": {"x": 300.0, "y": 0.0}, "rotation": 0.0}),
        ),
        (
            "pair-rotated",
            json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0}),
        ),
    ] {
        let (mut engine, seed) = seed(&synthetic_input, "left");
        let source = part(&seed, "left/U1");
        let reply = create(
            &mut engine,
            &seed,
            pair_id,
            "left",
            new_board("mirror", "mirror"),
            map.clone(),
        )
        .unwrap();
        let mirrored = part(&reply["document"], "mirror/U1");
        let expected = reflected_point(
            &map,
            (
                source["pose"]["at"]["x"].as_f64().unwrap(),
                source["pose"]["at"]["y"].as_f64().unwrap(),
            ),
        );
        assert_point(
            (
                mirrored["pose"]["at"]["x"].as_f64().unwrap(),
                mirrored["pose"]["at"]["y"].as_f64().unwrap(),
            ),
            expected,
            pair_id,
        );
        let source_heading = source["pose"]["rotation"].as_f64().unwrap();
        let expected_heading =
            (map["rotation"].as_f64().unwrap() + 180.0 - source_heading).rem_euclid(360.0);
        assert!(angles_match(
            mirrored["pose"]["rotation"].as_f64().unwrap(),
            expected_heading
        ));
        assert_eq!(mirrored["side"], source["side"]);
        assert_eq!(
            mirrored["generatorParameters"],
            source["generatorParameters"]
        );
    }
}

fn closure_positions(configuration: &Value) -> Vec<(f64, f64)> {
    let mut positions: Vec<(f64, f64)> = configuration["closureMounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["at"]["x"].as_f64().unwrap(),
                m["at"]["y"].as_f64().unwrap(),
            )
        })
        .collect();
    positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
    positions
}

#[test]
fn pairing_projects_authored_openings_and_closure_holes_to_the_mirror() {
    let synthetic_input = synthetic_half::with_construction_features(synthetic_half::document());
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    let document = &reply["document"];
    // Authored features move into pair ownership and remain effective on both halves.
    let record = pair(document, "pair-right");
    assert_eq!(record["features"].as_array().unwrap().len(), 2);
    for half in ["left", "right"] {
        let stored = &instance(document, half)["mechanical"];
        assert_eq!(stored["closureMounts"], json!([]), "{half}");
        assert_eq!(stored["openings"], json!([]), "{half}");
        assert_eq!(stored["boardId"], half, "{half}");
        assert_eq!(instance(document, half)["constructionOwnership"], "pair");
    }
    let source = effective(document, "left");
    let mirror = effective(document, "right");
    assert_eq!(source["openings"].as_array().unwrap().len(), 1);
    assert_eq!(mirror["openings"].as_array().unwrap().len(), 1);
    assert_eq!(closure_positions(&source), vec![(3.0, 5.0)]);
    assert_eq!(closure_positions(&mirror), vec![(197.0, 5.0)]);
    let opening_points = |configuration: &Value| {
        let mut points: Vec<_> = configuration["openings"][0]["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|vertex| {
                (
                    vertex["point"]["x"].as_f64().unwrap(),
                    vertex["point"]["y"].as_f64().unwrap(),
                )
            })
            .collect();
        points.sort_by(|a, b| a.partial_cmp(b).unwrap());
        points
    };
    assert_eq!(
        opening_points(&source),
        vec![(-4.0, -3.0), (-4.0, 3.0), (4.0, -3.0), (4.0, 3.0)]
    );
    assert_eq!(
        opening_points(&mirror),
        vec![(196.0, -3.0), (196.0, 3.0), (204.0, -3.0), (204.0, 3.0)]
    );
    assert_eq!(closure_holes(document, "right").len(), 1);
    assert_eq!(closure_holes(document, "left").len(), 1);
    let source_holes: BTreeSet<String> = closure_holes(document, "left")
        .iter()
        .map(|id| id.replace("/left/", "/right/"))
        .collect();
    assert!(source_holes.is_disjoint(&closure_holes(document, "right")));
    for hole in closure_holes(document, "right") {
        let board = document["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["id"] == "right")
            .unwrap();
        assert!(
            board["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| *id == hole),
            "{hole}"
        );
    }
    for hole in closure_holes(document, "left") {
        let board = document["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["id"] == "left")
            .unwrap();
        assert!(
            board["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| *id == hole),
            "{hole}"
        );
    }
}

#[test]
fn changed_ids_are_exactly_the_new_items_and_source_correspondences() {
    let synthetic_input = synthetic_half::with_construction_features(synthetic_half::document());
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    let document = &reply["document"];
    let record = &pair(document, "pair-right")["halves"];
    let mut expected: BTreeSet<String> = ["pair-right", "left", "right"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    for correspondence in record["correspondences"].as_array().unwrap() {
        expected.insert(correspondence["sourceId"].as_str().unwrap().to_owned());
        expected.insert(correspondence["mirrorId"].as_str().unwrap().to_owned());
    }
    // Generated clearance holes and their definitions that appeared or went away.
    let generated = |doc: &Value| -> BTreeSet<String> {
        let ids = |key: &str| {
            doc[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["id"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        };
        ids("parts")
            .into_iter()
            .chain(ids("definitions"))
            .filter(|id| id.starts_with("case-closure/") || id.starts_with("assembly-closure/"))
            .collect()
    };
    let (before, after) = (generated(&seed), generated(document));
    expected.extend(before.symmetric_difference(&after).cloned());
    let changed: BTreeSet<String> = reply["scene"]["changedIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(changed, expected);
    assert!(changed.contains("matrix/right-synthetic-keys/r0c0"));
    assert!(changed.contains("matrix/left-synthetic-keys/r0c0"));
    assert!(changed.contains("right/TRRS") && changed.contains("left/TRRS"));
    assert!(
        changed
            .iter()
            .any(|id| id.starts_with("case-closure/left/"))
    );
    assert!(
        changed
            .iter()
            .any(|id| id.starts_with("case-closure/right/"))
    );
}

#[test]
fn one_undo_step_removes_the_half_and_redo_restores_it() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let before = snapshot(&mut engine);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    let made = reply["document"]["revision"].as_u64().unwrap();
    assert_eq!(made, before["revision"].as_u64().unwrap() + 1);
    let mut undone = request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
    assert_eq!(undone["revision"].as_u64().unwrap(), made + 1);
    undone["revision"] = before["revision"].clone();
    assert_eq!(undone, before);
    let mut redone = request(&mut engine, json!({"id":"redo","kind":"redo"}))["document"].clone();
    redone["revision"] = reply["document"]["revision"].clone();
    assert_eq!(redone, reply["document"]);
    // The produced record reopens as a valid document.
    let (_, reopened) = open(&reply["document"]);
    assert_eq!(pair(&reopened, "pair-right")["constructionSource"], "left");
    assert_eq!(
        pair(&reopened, "pair-right")["halves"]["mirrorBoardId"],
        "right"
    );
}

/// A refused request names the problem and leaves the accepted document byte-identical.
fn refused(engine: &mut CoreEngine, seed: &Value, operation: Value) -> String {
    let before = serde_json::to_string(&snapshot(engine)).unwrap();
    let error = create_with(engine, seed, "commit", operation).unwrap_err();
    assert_eq!(
        serde_json::to_string(&snapshot(engine)).unwrap(),
        before,
        "{error}"
    );
    error
}

#[test]
fn refused_requests_name_the_problem_and_leave_the_document_byte_identical() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let good = synthetic_mapping(&synthetic_input);
    let op = |pair: &str, source: &str, mode: Value| operation(pair, source, mode, good.clone());
    assert!(
        refused(
            &mut engine,
            &seed,
            op("p", "left", new_board("right", "left"))
        )
        .contains("\"left\"")
    );
    assert!(
        refused(
            &mut engine,
            &seed,
            op("p", "nowhere", new_board("right", "right"))
        )
        .contains("nowhere")
    );
    assert!(
        refused(
            &mut engine,
            &seed,
            op("p", "left", new_board("left", "right"))
        )
        .contains("already exists")
    );
    assert!(
        refused(
            &mut engine,
            &seed,
            op(" ", "left", new_board("right", "right"))
        )
        .contains("pair named")
    );
    // An id the mirror would mint that another item already owns.
    assert!(
        refused(
            &mut engine,
            &seed,
            op("p", "left", new_board("left/U1", "x"))
        )
        .contains("already")
    );
    // The good request still succeeds after the refusals, and then cannot repeat.
    let (mut engine, seed) = open(&seed);
    let made = create_with(
        &mut engine,
        &seed,
        "commit",
        op("p", "left", new_board("right", "right")),
    )
    .unwrap();
    let after = made["document"].clone();
    assert!(
        refused(
            &mut engine,
            &after,
            op("p2", "left", new_board("second", "second"))
        )
        .contains("already belongs")
    );
    assert!(
        refused(
            &mut engine,
            &after,
            op("p", "right", new_board("third", "third"))
        )
        .contains("pair named")
    );
}

#[test]
fn a_closure_hole_without_a_catalogue_definition_is_refused_atomically() {
    let synthetic_input = synthetic_half::with_construction_features(synthetic_half::document());
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let mut op = operation(
        "pair-no-hole",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    );
    op["mountingHole"] = Value::Null;
    let error = refused(&mut engine, &seed, op);
    assert!(error.to_lowercase().contains("mounting"), "{error}");
}

#[test]
fn a_preview_changes_nothing_and_a_commit_still_follows() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let before = serde_json::to_string(&snapshot(&mut engine)).unwrap();
    let op = operation(
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    );
    let preview = create_with(&mut engine, &seed, "preview", op.clone()).unwrap();
    assert_eq!(preview["kind"], "preview");
    assert_eq!(
        serde_json::to_string(&snapshot(&mut engine)).unwrap(),
        before
    );
    let committed = create_with(&mut engine, &seed, "commit", op).unwrap();
    assert_eq!(committed["kind"], "scene");
}

#[test]
fn reuse_board_projects_authored_features_without_copying_board_data() {
    let synthetic_input = synthetic_half::with_construction_features(synthetic_half::document());
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-flipped",
        "left",
        json!({"kind":"reuse-board","instanceId":"flipped"}),
        mapping(0.0),
    )
    .unwrap();
    let document = &reply["document"];
    // Nothing is copied: boards, matrices, layouts, nets and every part but the generated holes.
    let owned = |doc: &Value, key: &str| -> Vec<Value> {
        doc[key]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| !v["id"].as_str().unwrap().starts_with("case-closure/"))
            .cloned()
            .collect()
    };
    assert_eq!(owned(document, "parts"), owned(&seed, "parts"));
    assert_eq!(document["matrices"], seed["matrices"]);
    assert_eq!(document["layouts"], seed["layouts"]);
    assert_eq!(document["nets"], seed["nets"]);
    assert_eq!(document["hardware"]["boards"], seed["hardware"]["boards"]);
    assert_eq!(document["boards"].as_array().unwrap().len(), 1);
    let flipped = instance(document, "flipped");
    assert_eq!(flipped["boardId"], "left");
    assert_eq!(flipped["half"], "right");
    assert_eq!(flipped["role"], "peripheral");
    assert_eq!(flipped["flipped"], true);
    assert_eq!(flipped["controllerPartId"], Value::Null);
    assert_eq!(flipped["constructionOwnership"], "pair");
    assert_eq!(instance(document, "left")["constructionOwnership"], "pair");
    // The linked features, not cleared copies: the flipped half reads the reflection.
    let record = pair(document, "pair-flipped");
    assert_eq!(record["members"], json!(["left", "flipped"]));
    assert_eq!(record["constructionSource"], "left");
    assert_eq!(record["halves"]["sourceBoardId"], "left");
    assert_eq!(record["halves"]["mirrorBoardId"], "left");
    assert_eq!(record["features"].as_array().unwrap().len(), 2);
    for half in ["left", "flipped"] {
        assert_eq!(
            instance(document, half)["mechanical"]["closureMounts"],
            json!([]),
            "{half}"
        );
        assert_eq!(
            instance(document, half)["mechanical"]["openings"],
            json!([]),
            "{half}"
        );
    }
    let (source, mirror) = (effective(document, "left"), effective(document, "flipped"));
    assert_eq!(source["openings"].as_array().unwrap().len(), 1);
    assert_eq!(mirror["openings"].as_array().unwrap().len(), 1);
    assert_eq!(closure_positions(&source), vec![(3.0, 5.0)]);
    assert_eq!(closure_positions(&mirror), vec![(-3.0, 5.0)]);
    assert_eq!(closure_holes(document, "left").len(), 1);
    // Reuse projects one physical board, so the reflected effective feature is not a second PCB part.
    assert!(closure_holes(document, "flipped").is_empty());
    let shared_board = document["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["id"] == "left")
        .unwrap();
    let shared_hole = closure_holes(document, "left").into_iter().next().unwrap();
    assert!(
        shared_board["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == &shared_hole)
    );
    // A reused board is one half of one record.
    let (_, reopened) = open(document);
    assert_eq!(pair(&reopened, "pair-flipped")["id"], "pair-flipped");
    assert!(
        refused(
            &mut engine,
            document,
            operation(
                "again",
                "left",
                json!({"kind":"reuse-board","instanceId":"again"}),
                mapping(0.0)
            )
        )
        .contains("already")
    );
}

#[test]
fn parts_keep_their_side_and_every_generator_parameter_exactly() {
    let synthetic_input = synthetic_half::document();
    let mut doc = seed_document(&synthetic_input, "left");
    for part in doc["parts"].as_array_mut().unwrap() {
        match part["id"].as_str().unwrap() {
            "left/U1" => part["generatorParameters"] = json!({"reversible": false, "custom": 9}),
            "left/RST" => {
                part["side"] = json!("back");
                part["generatorParameters"] = json!({"reversible": true});
            }
            _ => {}
        }
    }
    let (mut engine, seed) = open(&doc);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    let document = &reply["document"];
    for (source, mirror) in [
        ("left/U1", "right/U1"),
        ("left/RST", "right/RST"),
        ("left/TRRS", "right/TRRS"),
    ] {
        assert_eq!(
            part(document, source)["side"],
            part(document, mirror)["side"],
            "{mirror}"
        );
        assert_eq!(
            part(document, source)["generatorParameters"],
            part(document, mirror)["generatorParameters"],
            "{mirror}"
        );
    }
    assert_eq!(
        part(document, "right/U1")["generatorParameters"],
        json!({"reversible": false, "custom": 9})
    );
    assert_eq!(
        part(document, "right/RST")["generatorParameters"],
        json!({"reversible": true})
    );
    assert_eq!(part(document, "right/RST")["side"], "back");
    // The source is untouched too.
    assert_eq!(
        part(document, "left/U1")["generatorParameters"],
        json!({"reversible": false, "custom": 9})
    );
}

#[test]
fn layouts_keycaps_and_electrical_configuration_follow_the_mapped_ids() {
    let synthetic_input = synthetic_half::document();
    let mut doc = seed_document(&synthetic_input, "left");
    doc["keycaps"] = json!({
        "boards": {"left": {"color": "#111111", "legendColor": "#222222", "clearance": 0.6}},
        "matrices": {"left-synthetic-keys": {"profile": "dsa", "mount": "mx", "firstRow": 2, "wallThickness": 1.4}},
        "keys": {"matrix/left-synthetic-keys/r0c0": {"legend": "Q", "color": "#ff0000", "row": 3}},
    });
    doc["layouts"][0]["partIds"] = json!(["left/U1", "left/RST"]);
    doc["nets"][0]["id"] = json!("left/column/0/0");
    doc["nets"][0]["name"] = json!("LEFT/column/0/0");
    doc["nets"][0]["pins"] = json!([{"partId":"matrix/left-synthetic-keys/r0c0","padId":"pad-1"}]);
    doc["boards"][0]["netIds"][0] = json!("left/column/0/0");
    let board = &mut doc["hardware"]["boards"][0];
    board["locks"] = json!({"row/0": "P10", "peripheral/left/TRRS/split-rx": "P14"});
    board["assignments"] =
        json!({"row/0":"P3", "column/0":"P4", "peripheral/left/TRRS/split-rx":"P14"});
    board["keyBindings"] = json!({"matrix/left-synthetic-keys/r0c0": "&kp Q"});
    board["jumperStates"] = json!({});
    board["protectedHandoff"] =
        json!({"fingerprint": "f", "revision": 1, "assignments": {"row/0": "P10"}});
    let (mut engine, seed) = open(&doc);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    let document = &reply["document"];
    // Layouts: mapped ids and board, parts in order, no layout link, the source's untouched.
    let layout = |id: &str| {
        document["layouts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["id"] == id)
            .unwrap()
            .clone()
    };
    let expected = json!({"boardId": "right", "id": "right-synthetic-keys-layout", "matrixId": "right-synthetic-keys", "name": "Synthetic keys", "partIds": ["right/U1", "right/RST"]});
    assert_eq!(layout("right-synthetic-keys-layout"), expected);
    assert_eq!(layout("left-synthetic-keys-layout"), seed["layouts"][0]);
    assert_eq!(document["layouts"].as_array().unwrap().len(), 2);
    assert_eq!(
        layout("left-synthetic-keys-layout")["partIds"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(document["nets"].as_array().unwrap().len(), 8);
    let mapped_net = document["nets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|net| net["id"] == "right/column/0/0")
        .unwrap();
    assert_eq!(mapped_net["name"], "RIGHT/column/0/0");
    assert_eq!(
        mapped_net["pins"],
        json!([{"partId":"matrix/right-synthetic-keys/r0c0","padId":"pad-1"}])
    );
    // Keycaps: settings follow the mapped ids; a legend is copied as it is (a binding is not).
    let keycaps = &document["keycaps"];
    assert_eq!(keycaps["boards"]["right"], keycaps["boards"]["left"]);
    assert_eq!(
        keycaps["matrices"]["right-synthetic-keys"],
        keycaps["matrices"]["left-synthetic-keys"]
    );
    assert_eq!(
        keycaps["keys"]["matrix/right-synthetic-keys/r0c0"]["color"],
        "#ff0000"
    );
    assert_eq!(
        keycaps["keys"]["matrix/right-synthetic-keys/r0c0"]["row"],
        3
    );
    assert_eq!(
        keycaps["keys"]["matrix/right-synthetic-keys/r0c0"]["legend"],
        "Q"
    );
    assert_eq!(
        keycaps["keys"]["matrix/left-synthetic-keys/r0c0"]["legend"],
        "Q"
    );
    // Electrical: the same pins on the renamed functions, nothing of the source in the names.
    let boards = document["hardware"]["boards"].as_array().unwrap();
    let (left, right) = (&boards[0], &boards[1]);
    assert_eq!(right["boardId"], "right");
    assert_eq!(right["controllerPartId"], "right/U1");
    assert_eq!(
        right["assignments"],
        json!({"row/0":"P3", "column/0":"P4", "peripheral/right/TRRS/split-rx":"P14"})
    );
    assert_eq!(right["keyBindings"], json!({}));
    assert_eq!(right["protectedHandoff"], Value::Null);
    assert_eq!(
        left["keyBindings"],
        json!({"matrix/left-synthetic-keys/r0c0": "&kp Q"})
    );
    assert_eq!(
        right["locks"],
        json!({"row/0": "P10", "peripheral/right/TRRS/split-rx": "P14"})
    );
    assert_eq!(
        left["keyBindings"]["matrix/left-synthetic-keys/r0c0"],
        "&kp Q"
    );
    assert_eq!(right["keyBindings"], json!({}));
}

#[test]
fn a_board_with_nothing_on_it_and_a_matrix_id_without_the_board_token_still_mirror() {
    // An empty board mirrors to an empty board.
    let synthetic_input = synthetic_half::document();
    let mut doc = seed_document(&synthetic_input, "left");
    doc["parts"] = json!([]);
    doc["matrices"] = json!([]);
    doc["layouts"] = json!([]);
    doc["outline"] = json!([]);
    doc["nets"] = json!([]);
    doc["boards"][0]["partIds"] = json!([]);
    doc["boards"][0]["outlineIds"] = json!([]);
    doc["boards"][0]["netIds"] = json!([]);
    doc["hardware"]["boards"] = json!([]);
    let (mut engine, seed) = open(&doc);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    assert_eq!(reply["document"]["boards"].as_array().unwrap().len(), 2);
    // A matrix whose id does not contain the board id follows its members.
    let mut doc = seed_document(&synthetic_input, "left");
    let text = serde_json::to_string(&doc)
        .unwrap()
        .replace("left-synthetic-keys", "alpha");
    doc = serde_json::from_str(&text).unwrap();
    let (mut engine, seed) = open(&doc);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    let document = &reply["document"];
    assert!(
        document["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "right-alpha")
    );
    assert!(
        document["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "matrix/right-alpha/r0c0")
    );
    assert_reflects_every_part(document, "pair-right", "token-free matrix ids");
}

#[test]
fn reuse_board_refuses_a_map_other_than_the_flip_itself() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let reuse = json!({"kind":"reuse-board","instanceId":"flipped"});
    for map in [
        json!({"at": {"x": 12.0, "y": 0.0}, "rotation": 0.0}),
        json!({"at": {"x": 0.0, "y": 3.0}, "rotation": 0.0}),
        json!({"at": {"x": 0.0, "y": 0.0}, "rotation": 90.0}),
    ] {
        let error = refused(
            &mut engine,
            &seed,
            operation("p", "left", reuse.clone(), map),
        );
        assert!(
            error.contains("flipped instance") && error.contains("plain reflection"),
            "{error}"
        );
    }
}

/// One typed outline edit on `board`'s outline, returning the accepted document.
fn outline_edit(
    engine: &mut CoreEngine,
    document: &Value,
    target: Value,
    change: Value,
    seed: u64,
) -> Value {
    let reply: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"outline","kind":"edit","command":{
                "baseRevision": document["revision"], "transactionId": "outline", "phase": "commit",
                "targetIds": [], "operation": {"kind": "apply-outline-edit",
                    "edit": {"target": target, "change": change, "seed": seed}}}})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(reply["kind"], "scene", "{reply}");
    reply["document"].clone()
}

/// A point in a part's frame as the document places it: side-aware, turned by the part's heading.
fn world_of(document: &Value, part_id: &str, at: &Value) -> (f64, f64) {
    let part = part(document, part_id);
    let x = at["x"].as_f64().unwrap() * if part["side"] == "back" { -1.0 } else { 1.0 };
    let y = at["y"].as_f64().unwrap();
    let (sin, cos) = part["pose"]["rotation"]
        .as_f64()
        .unwrap()
        .to_radians()
        .sin_cos();
    (
        part["pose"]["at"]["x"].as_f64().unwrap() + x * cos - y * sin,
        part["pose"]["at"]["y"].as_f64().unwrap() + x * sin + y * cos,
    )
}

/// The pair map applied to a point: `q = R(rotation) * (-p.x, p.y) + at`.
fn reflected_point(map: &Value, p: (f64, f64)) -> (f64, f64) {
    let (sin, cos) = map["rotation"].as_f64().unwrap().to_radians().sin_cos();
    (
        -p.0 * cos - p.1 * sin + map["at"]["x"].as_f64().unwrap(),
        -p.0 * sin + p.1 * cos + map["at"]["y"].as_f64().unwrap(),
    )
}

fn assert_point(actual: (f64, f64), expected: (f64, f64), label: &str) {
    assert!(
        (actual.0 - expected.0).abs() < 1e-9 && (actual.1 - expected.1).abs() < 1e-9,
        "{label}: {actual:?} vs {expected:?}"
    );
}

/// Every outline identity in the document: features, connections, gaps, bridges and versions.
fn outline_identities(document: &Value) -> Vec<String> {
    fn feature(ids: &mut Vec<String>, f: &Value) {
        ids.push(f["id"].as_str().unwrap().to_owned());
        for c in f["connections"].as_array().into_iter().flatten() {
            ids.push(c["id"].as_str().unwrap().to_owned());
        }
        for g in f["settings"]["repair"]["keepGaps"]
            .as_array()
            .into_iter()
            .flatten()
        {
            ids.push(g["id"].as_str().unwrap().to_owned());
        }
    }
    let mut ids = Vec::new();
    for f in document["outline"].as_array().unwrap() {
        feature(&mut ids, f);
    }
    for state in document["boardOutlines"].as_array().into_iter().flatten() {
        for v in state["versions"].as_array().unwrap() {
            ids.push(v["id"].as_str().unwrap().to_owned());
            for f in v["geometry"]["features"].as_array().unwrap() {
                feature(&mut ids, f);
            }
            for g in v["geometry"]["protectedGaps"]
                .as_array()
                .into_iter()
                .flatten()
            {
                ids.push(g["id"].as_str().unwrap().to_owned());
            }
            for b in v["geometry"]["bridges"].as_array().into_iter().flatten() {
                ids.push(b["id"].as_str().unwrap().to_owned());
            }
        }
    }
    ids
}

#[test]
fn outlines_reflect_anchored_geometry_in_the_part_frame_and_keep_unique_identities() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, mut document) = seed(&synthetic_input, "left");
    let board = json!({"kind": "board", "boardId": "left"});
    let live = json!({"kind": "board", "boardId": "left"});
    // Live outline: an authored connection and a kept gap, some points attached to the controller.
    document = outline_edit(
        &mut engine,
        &document,
        board.clone(),
        json!({"add-connection": {"connection": {
        "id": "conn-live", "width": 3.0,
        "points": [{"at": {"x": 4.0, "y": 2.0}, "partId": "left/U1"}, {"at": {"x": 30.0, "y": -20.0}}]}}}),
        1,
    );
    document = outline_edit(
        &mut engine,
        &document,
        live,
        json!({"set-protected-gap": {"gap_id": "gap-live", "points": [
        {"at": {"x": 1.0, "y": 2.0}, "partId": "left/U1"}, {"at": {"x": 6.0, "y": 2.0}, "partId": "left/U1"},
        {"at": {"x": 6.0, "y": 8.0}, "partId": "left/U1"}]}}),
        2,
    );
    // A fixed copy with an anchored cutout and an anchored polygon.
    document = outline_edit(
        &mut engine,
        &document,
        board,
        json!({"copy": {"version_id": "fixed", "name": "Fixed"}}),
        3,
    );
    let version = json!({"kind": "version", "boardId": "left", "versionId": "fixed"});
    document = outline_edit(
        &mut engine,
        &document,
        version.clone(),
        json!({"add-feature": {"feature": {
        "kind": "polygon", "id": "cut-poly", "anchorPartId": "left/U1",
        "points": [{"x": 0.0, "y": 0.0}, {"x": 5.0, "y": 0.0}, {"x": 5.0, "y": 7.0}], "operation": "subtract"}}}),
        5,
    );
    document = outline_edit(
        &mut engine,
        &document,
        version,
        json!({"set-protected-gap": {"gap_id": "gap-fixed", "points": [
        {"at": {"x": 40.0, "y": -40.0}}, {"at": {"x": 50.0, "y": -40.0}}, {"at": {"x": 50.0, "y": -30.0}}]}}),
        6,
    );
    // A version-local edit can only add polygons; an anchored cutout reaches a version from older
    // data, so it is placed in the stored document directly.
    document["boardOutlines"][0]["versions"][0]["geometry"]["features"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind": "rect", "id": "cut-rect", "anchorPartId": "left/RST", "center": {"x": 2.0, "y": 3.0},
            "size": {"x": 4.0, "y": 5.0}, "radius": 0.0, "rotation": 20.0, "operation": "subtract"}));
    let (mut engine, seed_doc) = open(&document);
    let map = json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0});
    let reply = create(
        &mut engine,
        &seed_doc,
        "pair-right",
        "left",
        new_board("right", "right"),
        map.clone(),
    )
    .unwrap();
    let out = &reply["document"];
    let find = |list: &Value, id: &str| {
        list.as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == id)
            .unwrap_or_else(|| panic!("missing {id}; available: {list}"))
            .clone()
    };

    // Live outline, in the part frame: U1 is an independent part, so the mirror turns it by 180
    // degrees (ticket 30) and its attached point is (x, -y) in that frame; the free one maps.
    let live = find(&out["outline"], "right/synthetic-envelope");
    let connection = &live["connections"][0];
    assert_eq!(connection["points"][0]["partId"], "right/U1");
    assert_eq!(connection["points"][0]["at"], json!({"x": 4.0, "y": -2.0}));
    assert_point(
        world_of(out, "right/U1", &connection["points"][0]["at"]),
        reflected_point(&map, world_of(out, "left/U1", &json!({"x": 4.0, "y": 2.0}))),
        "attached connection point",
    );
    let free = &connection["points"][1];
    assert!(free["partId"].is_null());
    assert_point(
        (
            free["at"]["x"].as_f64().unwrap(),
            free["at"]["y"].as_f64().unwrap(),
        ),
        reflected_point(&map, (30.0, -20.0)),
        "free connection point",
    );
    let gap = &live["settings"]["repair"]["keepGaps"][0];
    for (index, source) in [
        (0, json!({"x": 1.0, "y": 2.0})),
        (1, json!({"x": 6.0, "y": 2.0})),
        (2, json!({"x": 6.0, "y": 8.0})),
    ] {
        assert_eq!(gap["points"][index]["partId"], "right/U1");
        assert_point(
            world_of(out, "right/U1", &gap["points"][index]["at"]),
            reflected_point(&map, world_of(out, "left/U1", &source)),
            "kept gap point",
        );
    }

    // The fixed copy: anchored cutouts reflect in the part frame, world geometry through the map.
    let state = out["boardOutlines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["boardId"] == "right")
        .unwrap();
    assert_eq!(state["activeVersionId"], "right/fixed");
    let geometry = &state["versions"][0]["geometry"];
    let rect = find(&geometry["features"], "right/cut-rect");
    assert_eq!(rect["anchorPartId"], "right/RST");
    assert_point(
        world_of(out, "right/RST", &rect["center"]),
        reflected_point(
            &map,
            world_of(out, "left/RST", &json!({"x": 2.0, "y": 3.0})),
        ),
        "anchored rect centre",
    );
    assert_eq!(rect["rotation"].as_f64().unwrap(), -20.0);
    let polygon = find(&geometry["features"], "right/cut-poly");
    let source_points = [(0.0, 0.0), (5.0, 0.0), (5.0, 7.0)];
    let mirrored_world: Vec<(f64, f64)> = polygon["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| world_of(out, "right/U1", p))
        .collect();
    for (x, y) in source_points {
        let expected = reflected_point(&map, world_of(out, "left/U1", &json!({"x": x, "y": y})));
        assert!(
            mirrored_world
                .iter()
                .any(|w| (w.0 - expected.0).abs() < 1e-9 && (w.1 - expected.1).abs() < 1e-9),
            "{x},{y}"
        );
    }
    // Reflection reverses the winding, so the points come back in the reverse order. U1 is an
    // independent part turned by 180 degrees, so the local point is (x, -y), not (-x, y).
    assert_eq!(polygon["points"][0], json!({"x": 5.0, "y": -7.0}));
    let fixed_gap = &geometry["protectedGaps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == "right/gap-fixed")
        .unwrap();
    assert_point(
        (
            fixed_gap["points"][0]["at"]["x"].as_f64().unwrap(),
            fixed_gap["points"][0]["at"]["y"].as_f64().unwrap(),
        ),
        reflected_point(&map, (40.0, -40.0)),
        "protected gap",
    );

    // One namespace: the only repeated identity is the gap kept on the live outline and
    // protected in the fixed copy, which is one gap with one identity on each half.
    let ids = outline_identities(out);
    let repeated: BTreeSet<&String> = ids
        .iter()
        .filter(|id| ids.iter().filter(|other| other == id).count() > 1)
        .collect();
    let expected: BTreeSet<String> = ["gap-live", "right/gap-live"].map(str::to_owned).into();
    assert_eq!(
        repeated,
        expected.iter().collect::<BTreeSet<_>>(),
        "{ids:?}"
    );
    assert!(ids.iter().any(|id| id == "right/conn-live") && ids.iter().any(|id| id == "conn-live"));
    assert_eq!(
        find(&out["outline"], "left/synthetic-envelope"),
        find(&seed_doc["outline"], "left/synthetic-envelope")
    );
}

#[test]
fn a_mirrored_outline_identity_that_is_already_used_refuses_the_whole_record() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, mut document) = seed(&synthetic_input, "left");
    document = outline_edit(
        &mut engine,
        &document,
        json!({"kind": "board", "boardId": "left"}),
        json!({"add-connection": {"connection": {
        "id": "left-link", "width": 3.0,
        "points": [{"at": {"x": 4.0, "y": 2.0}, "partId": "left/U1"}, {"at": {"x": 30.0, "y": -20.0}}]}}}),
        1,
    );
    // Another top-level feature already owns the id the mirror's connection would take.
    let mut stray = document.clone();
    stray["outline"].as_array_mut().unwrap().push(
        json!({"kind": "polygon", "id": "right-link", "operation": "add",
        "points": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 0.0}, {"x": 1.0, "y": 1.0}]}),
    );
    let (mut engine, stray) = open(&stray);
    let error = refused(
        &mut engine,
        &stray,
        operation(
            "p",
            "left",
            new_board("right", "right"),
            synthetic_mapping(&synthetic_input),
        ),
    );
    assert!(error.contains("right-link"), "{error}");
}

/// The VIK DRV2605L reference circuit, imported the way `module_circuit_electrical` imports it.
const HAPTIC_SOURCE: &str = include_str!(
    "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/haptic-drv2605l/haptic-drv2605l.kicad_pcb"
);

fn imported_haptic() -> Value {
    let source = json!({
        "id":"import", "kind":"import-module-board", "definitionId":"haptic",
        "name":"DRV2605L reference circuit", "source":HAPTIC_SOURCE,
        "provenance":{"repository":"https://github.com/sadekbaroudi/vik",
            "revision":"cd5d16e4cd9137a229fc673412a89d75f4e64553",
            "path":"pcb/haptic-drv2605l/haptic-drv2605l.kicad_pcb",
            "license":"CERN-OHL-S-2.0"},
        "family":"feedback", "variant":"3V3 pullups with JP1 bridged",
        "repair":"drv2605l-pullups3v3"
    });
    let reply: Value =
        serde_json::from_str(&boardstudio_core::artifact::request(&source.to_string())).unwrap();
    assert_eq!(reply["kind"], "import-module-board", "{reply}");
    reply["result"].clone()
}

/// Embeds the haptic circuit as `id` on `board`, joining the given ports to host nets, and returns
/// the document after the edit.
fn embed_haptic(
    engine: &mut CoreEngine,
    document: &Value,
    board: &str,
    id: &str,
    joins: Value,
) -> Value {
    let reply = request(
        engine,
        json!({"id": format!("embed-{id}"), "kind": "edit", "command": {
            "baseRevision": document["revision"], "transactionId": format!("embed-{id}"),
            "phase": "commit", "targetIds": [id], "operation": {
                "kind": "embed-module-circuit", "id": id, "definition": imported_haptic(),
                "hostBoardId": board, "pose": {"at": {"x": 120.0, "y": 40.0}, "rotation": 0.0},
                "side": "front", "joins": joins}}}),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    reply["document"].clone()
}

/// The net ids of `board`, in the board's order.
fn board_nets(document: &Value, board: &str) -> Vec<String> {
    document["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == board)
        .unwrap()["netIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_owned())
        .collect()
}

fn record<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["embeddedCircuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("embedded circuit {id} is missing"))
}

fn definition<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == id)
        .unwrap_or_else(|| panic!("definition {id} is missing"))
}

fn net<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["nets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()
}

/// The record's correspondences of one kind as (source id, mirror id) pairs.
fn pairs_of_kind(document: &Value, pair_id: &str, kind: &str) -> BTreeSet<(String, String)> {
    pair(document, pair_id)["halves"]["correspondences"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["kind"] == kind)
        .map(|c| {
            (
                c["sourceId"].as_str().unwrap().to_owned(),
                c["mirrorId"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// An embedded circuit mirrors with its namespace intact (ticket 28). `left-haptic` carries a board
/// token, so it mirrors as `right-haptic`; `review/haptic` carries none, so it mirrors as
/// `right/review/haptic`. Under the board-prefix rule alone a member would become
/// `right/embedded/...`, outside the `embedded/<id>/` namespace that `modules::circuit` reads. Each
/// member and private net moves under its circuit's mirror namespace, each member keeps its side and
/// generator parameters, takes the pair map and the full heading rule, and each copied definition
/// is the source's with only its id changed. A port that joins a host net mirrors to the host
/// mirror net.
#[test]
fn a_board_with_embedded_circuits_mirrors_with_valid_namespaces_and_records() {
    let synthetic_input = synthetic_half::document();
    for map in [
        mapping(300.0),
        json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0}),
    ] {
        let (mut engine, seed) = seed(&synthetic_input, "left");
        // The first host net of the left board stands for a host net the circuit joins.
        let joined = board_nets(&seed, "left")[0].clone();
        let one = embed_haptic(
            &mut engine,
            &seed,
            "left",
            "left-haptic",
            json!({"gnd": joined}),
        );
        let source = embed_haptic(&mut engine, &one, "left", "review/haptic", json!({}));
        let before = snapshot(&mut engine);
        let reply = create(
            &mut engine,
            &source,
            "pair-right",
            "left",
            new_board("right", "right"),
            map.clone(),
        )
        .unwrap();
        let document = &reply["document"];
        let rotation = map["rotation"].as_f64().unwrap();
        let mirror_joined = pairs_of_kind(document, "pair-right", "net")
            .into_iter()
            .find(|(source, _)| *source == joined)
            .expect("the joined host net has a mirror")
            .1;
        let at = |p: &Value| {
            (
                p["pose"]["at"]["x"].as_f64().unwrap(),
                p["pose"]["at"]["y"].as_f64().unwrap(),
            )
        };

        for (source_id, mirror_id) in [
            ("left-haptic", "right-haptic"),
            ("review/haptic", "right/review/haptic"),
        ] {
            let original = record(&source, source_id);
            let mirrored = record(document, mirror_id);
            let from = format!("embedded/{source_id}/");
            let to = format!("embedded/{mirror_id}/");
            let mapped = |id: &str| -> String {
                id.strip_prefix(&from)
                    .map_or_else(|| id.to_owned(), |rest| format!("{to}{rest}"))
            };
            assert_eq!(mirrored["hostBoardId"], "right", "{mirror_id}");
            assert_eq!(
                mirrored["definitionId"], original["definitionId"],
                "{mirror_id}"
            );

            let source_parts: Vec<&str> = original["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_str().unwrap())
                .collect();
            let mirror_parts: Vec<String> = source_parts.iter().map(|p| mapped(p)).collect();
            assert_eq!(
                mirrored["partIds"],
                json!(mirror_parts),
                "{mirror_id} parts"
            );
            assert!(
                mirror_parts.iter().all(|p| p.starts_with(&to)),
                "{mirror_id}: a member is outside {to}"
            );

            // Private nets move under the namespace; the joined host ground maps to the host mirror.
            let mut mirror_nets = Vec::new();
            for net_id in original["netIds"].as_array().unwrap() {
                let net_id = net_id.as_str().unwrap();
                if net_id.starts_with(&from) {
                    mirror_nets.push(mapped(net_id));
                } else {
                    assert_eq!(
                        net_id, joined,
                        "{mirror_id}: only the joined net is a host net"
                    );
                    mirror_nets.push(mirror_joined.clone());
                }
            }
            assert_eq!(mirrored["netIds"], json!(mirror_nets), "{mirror_id} nets");
            let mirror_ports: serde_json::Map<String, Value> = original["ports"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(role, net_id)| {
                    let net_id = net_id.as_str().unwrap();
                    let mirrored_net = if net_id == joined {
                        mirror_joined.clone()
                    } else {
                        mapped(net_id)
                    };
                    (role.clone(), json!(mirrored_net))
                })
                .collect();
            assert_eq!(
                mirrored["ports"],
                Value::Object(mirror_ports),
                "{mirror_id} ports"
            );

            for (source_part, mirror_part) in source_parts.iter().zip(&mirror_parts) {
                let (s, m) = (part(&source, source_part), part(document, mirror_part));
                assert_eq!(m["side"], s["side"], "{mirror_part} side");
                assert_eq!(
                    m["generatorParameters"], s["generatorParameters"],
                    "{mirror_part} parameters"
                );
                // The component definition moves with the namespace and keeps its content.
                let (source_def, mirror_def) = (
                    s["definitionId"].as_str().unwrap(),
                    mapped(s["definitionId"].as_str().unwrap()),
                );
                assert_eq!(
                    m["definitionId"],
                    json!(mirror_def),
                    "{mirror_part} definition"
                );
                let mut expected_def = definition(&source, source_def).clone();
                expected_def["id"] = json!(mirror_def);
                if expected_def["mechanicalProfile"].is_object() {
                    expected_def["mechanicalProfile"]["definitionId"] = json!(mirror_def);
                }
                assert_eq!(
                    definition(document, &mirror_def),
                    &expected_def,
                    "{mirror_part} definition copy"
                );
                // Position through the pair map, heading by the full rule (180 - r on an axis map).
                let source_at = at(s);
                let expected_at = reflected_point(&map, source_at);
                assert_point(at(m), expected_at, mirror_part);
                let heading = s["pose"]["rotation"].as_f64().unwrap();
                assert!(
                    angles_match(
                        m["pose"]["rotation"].as_f64().unwrap(),
                        rotation + 180.0 - heading
                    ),
                    "{mirror_part}: {} from {heading} under map {rotation}",
                    m["pose"]["rotation"]
                );
            }

            // Each private net keeps its pins on the mirrored members.
            for net_id in original["netIds"].as_array().unwrap() {
                let net_id = net_id.as_str().unwrap();
                if !net_id.starts_with(&from) {
                    continue;
                }
                let pins: Vec<Value> = net(&source, net_id)["pins"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|pin| json!({"partId": mapped(pin["partId"].as_str().unwrap()), "padId": pin["padId"]}))
                    .collect();
                assert_eq!(
                    net(document, &mapped(net_id))["pins"],
                    json!(pins),
                    "{mapped_net} pins",
                    mapped_net = mapped(net_id)
                );
            }
            // The mirror of the joined host net carries every member pin this circuit puts on it.
            if original["ports"]["gnd"] == joined.as_str() {
                let mirror_pins = net(document, &mirror_joined)["pins"].as_array().unwrap();
                for pin in net(&source, &joined)["pins"].as_array().unwrap() {
                    let partid = pin["partId"].as_str().unwrap();
                    if partid.starts_with(&from) {
                        let expected = json!({"partId": mapped(partid), "padId": pin["padId"]});
                        assert!(
                            mirror_pins.contains(&expected),
                            "{mirror_id}: {expected} on the mirror of the joined net"
                        );
                    }
                }
            }
            assert!(
                pairs_of_kind(document, "pair-right", "embedded-circuit")
                    .contains(&(source_id.to_owned(), mirror_id.to_owned())),
                "{mirror_id} correspondence"
            );
        }

        // No mirror id leaves the namespace, and the part and circuit correspondences are recorded.
        assert!(
            !document.to_string().contains("right/embedded/"),
            "a mirror id left the namespace"
        );
        assert_eq!(
            pairs_of_kind(document, "pair-right", "embedded-circuit"),
            [
                ("left-haptic", "right-haptic"),
                ("review/haptic", "right/review/haptic"),
            ]
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .into()
        );
        assert!(pairs_of_kind(document, "pair-right", "part").contains(&(
            "embedded/left-haptic/C1".into(),
            "embedded/right-haptic/C1".into()
        )));

        // Every new circuit member, net and definition is reported as changed.
        let changed: BTreeSet<String> = reply["scene"]["changedIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect();
        for id in [
            "right-haptic",
            "right/review/haptic",
            "embedded/right-haptic/C1",
        ] {
            assert!(changed.contains(id), "{id} missing from {changed:?}");
        }
        // The whole edit is one Undo step, and the record reopens as a valid document.
        let mut undone =
            request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
        undone["revision"] = before["revision"].clone();
        assert_eq!(undone, before);
        let (_, reopened) = open(document);
        assert_eq!(record(&reopened, "right-haptic")["hostBoardId"], "right");
    }
}

/// A board with a mounted module and an embedded circuit mirrors both: the module by ticket 27's
/// rule, the circuit by ticket 28's, and the record names each of them.
#[test]
fn a_circuit_and_a_module_on_one_board_mirror_together() {
    let synthetic_input = synthetic_half::document();
    let mut source = seed_document(&synthetic_input, "left");
    add_two_splitters(&mut source, "left", "left");
    let (mut engine, seed) = open(&source);
    let with_circuit = embed_haptic(&mut engine, &seed, "left", "left-haptic", json!({}));
    let reply = create(
        &mut engine,
        &with_circuit,
        "pair-right",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap();
    let document = &reply["document"];
    let expected_modules: BTreeSet<(String, String)> = [
        ("left/splitter-above", "right/splitter-above"),
        ("left/splitter-below", "right/splitter-below"),
    ]
    .map(|(a, b)| (a.to_owned(), b.to_owned()))
    .into();
    assert_eq!(module_pairs(document, "pair-right"), expected_modules);
    assert_eq!(
        pairs_of_kind(document, "pair-right", "embedded-circuit"),
        [("left-haptic", "right-haptic")]
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .into()
    );
    assert_eq!(
        module(document, "right/splitter-above")["connection"]["hostConnectorPartId"],
        "right/TRRS"
    );
    assert_eq!(record(document, "right-haptic")["hostBoardId"], "right");
}

/// A mirror circuit id that is already a circuit on the source board refuses the record by name, and
/// the document stays byte-identical.
#[test]
fn a_mirror_circuit_id_already_in_use_refuses_the_record_and_changes_nothing() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let one = embed_haptic(&mut engine, &seed, "left", "left-haptic", json!({}));
    let two = embed_haptic(&mut engine, &one, "left", "right-haptic", json!({}));
    let error = refused(
        &mut engine,
        &two,
        operation(
            "pair-right",
            "left",
            new_board("right", "right"),
            mapping(300.0),
        ),
    );
    assert!(
        error.contains("\"right-haptic\" already exists")
            && !error.contains("cannot be mirrored yet"),
        "{error}"
    );
}

/// The mirror of a board with embedded circuits is as valid as its source (ticket 28 verification):
/// each circuit finding of the source recurs on the mirror under the mirror circuit id with the same
/// severity and message, no finding is new, every finding target exists on the mirror, no part
/// reference repeats (a circuit member whose mirror reference a host part already holds takes the next
/// free one), and the same edit on the same source gives the same references.
#[test]
fn a_mirrored_circuit_reports_the_findings_of_its_source_and_no_reference_repeats() {
    let synthetic_input = synthetic_half::document();
    let build = || {
        let (mut engine, seed) = seed(&synthetic_input, "left");
        let joined = board_nets(&seed, "left")[0].clone();
        let one = embed_haptic(
            &mut engine,
            &seed,
            "left",
            "left-haptic",
            json!({"gnd": joined}),
        );
        let source = embed_haptic(&mut engine, &one, "left", "review/haptic", json!({}));
        let reply = create(
            &mut engine,
            &source,
            "pair-right",
            "left",
            new_board("right", "right"),
            mapping(300.0),
        )
        .unwrap();
        (source, reply)
    };
    let (source, reply) = build();
    let document = &reply["document"];
    let opened: Value = serde_json::from_str(
        &CoreEngine::new().request(&json!({"id":"o","kind":"open","document":source}).to_string()),
    )
    .unwrap();
    let circuit_findings =
        |scene: &Value, circuits: [&str; 2]| -> BTreeSet<(String, String, String)> {
            scene["findings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| {
                    circuits.iter().any(|c| {
                        f["id"]
                            .as_str()
                            .unwrap()
                            .starts_with(&format!("embedded/{c}/"))
                    })
                })
                .map(|f| {
                    (
                        f["id"].as_str().unwrap().to_owned(),
                        f["severity"].as_str().unwrap().to_owned(),
                        f["message"].as_str().unwrap().to_owned(),
                    )
                })
                .collect()
        };
    let on_source = circuit_findings(&opened["scene"], ["left-haptic", "review/haptic"]);
    let on_mirror = circuit_findings(&reply["scene"], ["right-haptic", "right/review/haptic"]);
    assert!(!on_source.is_empty(), "the fixture raises circuit findings");
    let renamed: BTreeSet<(String, String, String)> = on_source
        .iter()
        .map(|(id, severity, message)| {
            let id = id
                .replacen("embedded/left-haptic/", "embedded/right-haptic/", 1)
                .replacen(
                    "embedded/review/haptic/",
                    "embedded/right/review/haptic/",
                    1,
                );
            (id, severity.clone(), message.clone())
        })
        .collect();
    assert_eq!(on_mirror, renamed);

    let exists = |id: &str| {
        document["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == id)
            || document["embeddedCircuits"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["id"] == id)
    };
    for finding in reply["scene"]["findings"].as_array().unwrap() {
        if !finding["id"]
            .as_str()
            .unwrap()
            .starts_with("embedded/right")
        {
            continue;
        }
        for target in finding["targetIds"].as_array().unwrap() {
            assert!(exists(target.as_str().unwrap()), "{finding}");
        }
    }

    let references = |doc: &Value| -> Vec<(String, String)> {
        doc["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p["id"].as_str().unwrap().to_owned(),
                    p["reference"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    let all = references(document);
    let distinct: BTreeSet<&String> = all.iter().map(|(_, reference)| reference).collect();
    assert_eq!(
        distinct.len(),
        all.len(),
        "a part reference repeats: {all:?}"
    );
    let (_, again) = build();
    assert_eq!(
        references(&again["document"]),
        all,
        "references are deterministic"
    );
}

/// Reuse-board copies no circuit. An embedded circuit belongs to its board, not to an instance (the
/// model has no instance pin), so the flipped instance shows it without a copy.
#[test]
fn reuse_board_copies_no_embedded_circuit_because_it_belongs_to_the_board() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let with_circuit = embed_haptic(&mut engine, &seed, "left", "left-haptic", json!({}));
    let before = snapshot(&mut engine);
    let reply = create(
        &mut engine,
        &with_circuit,
        "pair-flip",
        "left",
        json!({"kind":"reuse-board","instanceId":"flipped"}),
        json!({"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}),
    )
    .unwrap();
    let document = &reply["document"];
    assert_eq!(document["embeddedCircuits"], before["embeddedCircuits"]);
    let circuit_parts = |doc: &Value| -> Vec<Value> {
        doc["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["id"].as_str().unwrap().starts_with("embedded/"))
            .cloned()
            .collect()
    };
    assert_eq!(circuit_parts(document), circuit_parts(&before));
    assert!(instance(document, "flipped")["flipped"].as_bool().unwrap());
    assert!(pairs_of_kind(document, "pair-flip", "embedded-circuit").is_empty());
}

/// The VIK module catalogue used by the synthetic module-operation test.
const MODULE_CATALOGUE: &str = include_str!("../../catalogue/modules/imported-modules.json");

fn module<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == id)
        .unwrap()
}

/// Puts two chained VIK splitters on `board`, from the synthetic test input places. The
/// first is pinned to physical instance `host` and sits on the front; the second is unpinned, on
/// the back, and takes the first's host-side output, so the chain is valid on the source.
fn add_two_splitters(document: &mut Value, board: &str, host: &str) {
    let library: Value = serde_json::from_str(MODULE_CATALOGUE).unwrap();
    let definition = library["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["row"] == "vik-splitter")
        .unwrap()["definition"]
        .clone();
    let connection = |bus: &str| {
        json!({
            "hostConnectorPartId": format!("{board}/TRRS"),
            "modulePortId": "J1002",
            "busId": format!("{board}/{bus}"),
            "assignments": {},
            "cableType": "type-a-12-0.5",
            "railVoltages": {},
        })
    };
    let mut downstream = connection("bus-b");
    downstream["upstreamModuleId"] = json!(format!("{board}/splitter-above"));
    downstream["upstreamPortId"] = json!("J1004");
    let module = |id: &str,
                  pinned: Option<&str>,
                  face: (&str, &str),
                  at: f64,
                  rotation: f64,
                  connection: Value| {
        let mut module = json!({
            "id": format!("{board}/{id}"),
            "definitionId": definition["id"],
            "hostBoardId": board,
            "hostFace": face.0,
            "facingFace": face.1,
            "at": {"x": at, "y": 20.0},
            "rotation": rotation,
            "gap": 3.0,
            "attachment": "board",
            "detached": false,
            "serviceClearance": 4.0,
            "connection": connection,
        });
        if let Some(instance) = pinned {
            module["hostInstanceId"] = json!(instance);
        }
        module
    };
    document["moduleDefinitions"] = json!([definition.clone()]);
    document["modules"] = json!([
        module(
            "splitter-above",
            Some(host),
            ("front", "back"),
            29.0,
            0.0,
            connection("bus-a")
        ),
        module(
            "splitter-below",
            None,
            ("back", "front"),
            57.0,
            135.0,
            downstream
        ),
    ]);
}

/// The source document with its second module changed by `edit`.
fn with_module_edit(source: &Value, edit: impl Fn(&mut Value)) -> Value {
    let mut document = source.clone();
    edit(&mut document["modules"][1]);
    document
}

/// The record's module correspondences as (source id, mirror id) pairs.
fn module_pairs(document: &Value, pair_id: &str) -> BTreeSet<(String, String)> {
    // An empty correspondence list is omitted from the record.
    pair(document, pair_id)["halves"]["correspondences"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["kind"] == "module")
        .map(|c| {
            (
                c["sourceId"].as_str().unwrap().to_owned(),
                c["mirrorId"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// A mounted module is mirrored by pose and id: `at` through the pair map, the heading through
/// the full rule `rotation + 180 - r` (never modulo 180, so the module is not flipped), and its
/// definition, faces, gap, attachment, supports and connection parameters unchanged. Every
/// reference is renamed by the board-prefix rule, and the host instance is the mirror instance.
#[test]
fn two_hosted_modules_mirror_with_exact_ids_poses_and_correspondences() {
    let synthetic_input = synthetic_half::document();
    for map in [
        mapping(300.0),
        json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0}),
    ] {
        let mut source = seed_document(&synthetic_input, "left");
        add_two_splitters(&mut source, "left", "left");
        let (mut engine, seed) = open(&source);
        let before = snapshot(&mut engine);
        let reply = create(
            &mut engine,
            &seed,
            "pair-right",
            "left",
            new_board("right", "right"),
            map.clone(),
        )
        .unwrap();
        let document = &reply["document"];
        let ids: BTreeSet<String> = document["modules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap().to_owned())
            .collect();
        let expected_ids: BTreeSet<String> = [
            "left/splitter-above",
            "left/splitter-below",
            "right/splitter-above",
            "right/splitter-below",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(ids, expected_ids, "map {map}");
        let expected_pairs: BTreeSet<(String, String)> = [
            ("left/splitter-above", "right/splitter-above"),
            ("left/splitter-below", "right/splitter-below"),
        ]
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
        .into();
        assert_eq!(module_pairs(document, "pair-right"), expected_pairs);

        let rotation = map["rotation"].as_f64().unwrap();
        for (source_id, mirror_id) in [
            ("left/splitter-above", "right/splitter-above"),
            ("left/splitter-below", "right/splitter-below"),
        ] {
            let original = module(&source, source_id);
            let mirror = module(document, mirror_id);
            let at = |m: &Value| {
                (
                    m["at"]["x"].as_f64().unwrap(),
                    m["at"]["y"].as_f64().unwrap(),
                )
            };
            assert_point(at(mirror), reflected_point(&map, at(original)), mirror_id);
            let heading = original["rotation"].as_f64().unwrap();
            assert!(
                angles_match(
                    mirror["rotation"].as_f64().unwrap(),
                    rotation + 180.0 - heading
                ),
                "{mirror_id}: {} from {heading} under map {rotation}",
                mirror["rotation"]
            );
            assert_eq!(mirror["hostBoardId"], "right", "{mirror_id}");
            for key in [
                "definitionId",
                "hostFace",
                "facingFace",
                "gap",
                "attachment",
                "detached",
                "serviceClearance",
                "mountSupports",
            ] {
                assert_eq!(mirror[key], original[key], "{mirror_id} {key}");
            }
            let (from, to) = (&original["connection"], &mirror["connection"]);
            assert_eq!(to["hostConnectorPartId"], "right/TRRS", "{mirror_id}");
            assert_eq!(to["modulePortId"], from["modulePortId"], "{mirror_id}");
            assert_eq!(to["cableType"], from["cableType"], "{mirror_id}");
            assert_eq!(to["assignments"], from["assignments"], "{mirror_id}");
            assert_eq!(to["railVoltages"], from["railVoltages"], "{mirror_id}");
        }
        // The pinned module moves to the mirror instance; the unpinned one stays unpinned.
        assert_eq!(
            module(document, "right/splitter-above")["hostInstanceId"],
            "right"
        );
        assert!(module(document, "right/splitter-below")["hostInstanceId"].is_null());
        // Bus ids and the chain are mapped by the same rule; the chain is the mirror's own.
        assert_eq!(
            module(document, "right/splitter-above")["connection"]["busId"],
            "right/bus-a"
        );
        assert_eq!(
            module(document, "right/splitter-below")["connection"]["busId"],
            "right/bus-b"
        );
        let downstream = &module(document, "right/splitter-below")["connection"];
        assert_eq!(downstream["upstreamModuleId"], "right/splitter-above");
        assert_eq!(downstream["upstreamPortId"], "J1004");
        // Every new id is reported, and the whole edit is one Undo step.
        let changed: BTreeSet<String> = reply["scene"]["changedIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect();
        for id in ["right/splitter-above", "right/splitter-below", "right"] {
            assert!(changed.contains(id), "{id} missing from {changed:?}");
        }
        let undone = request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
        assert_eq!(undone["modules"], before["modules"]);
        // An empty list is omitted from the document, so no pair is the same as none.
        assert!(
            undone["hardware"]["pairs"]
                .as_array()
                .into_iter()
                .flatten()
                .next()
                .is_none()
        );
    }
}

/// A module the mirror cannot place names the module and changes nothing: a pin to an instance
/// other than the source, a connector off the source board, and an upstream module off it.
#[test]
fn a_module_the_mirror_cannot_place_refuses_the_record_and_changes_nothing() {
    let synthetic_input = synthetic_half::document();
    let mut source = seed_document(&synthetic_input, "left");
    add_two_splitters(&mut source, "left", "left");
    type ModuleEdit = fn(&mut Value);
    let cases: [(&str, ModuleEdit, &str); 3] = [
        (
            "pinned elsewhere",
            |m| m["hostInstanceId"] = json!("elsewhere"),
            "elsewhere",
        ),
        (
            "connector off the board",
            |m| m["connection"]["hostConnectorPartId"] = json!("other/TRRS"),
            "other/TRRS",
        ),
        (
            "upstream off the board",
            |m| m["connection"]["upstreamModuleId"] = json!("other/splitter-above"),
            "other/splitter-above",
        ),
    ];
    for (label, edit, named) in cases {
        let document = with_module_edit(&source, edit);
        let (mut engine, seed) = open(&document);
        let error = refused(
            &mut engine,
            &seed,
            operation(
                "pair-right",
                "left",
                new_board("right", "right"),
                synthetic_mapping(&synthetic_input),
            ),
        );
        assert!(
            error.contains("splitter-below") && error.contains(named),
            "{label}: {error}"
        );
    }
}

/// Reuse-board copies no module: a module hosted on the board is reflected by the flipped view
/// (resolve), so the record holds no module correspondence. A module pinned to the source
/// instance is not visible from the flipped instance, so the operation refuses it by name.
#[test]
fn reuse_board_copies_no_module_and_refuses_one_pinned_to_the_source() {
    let synthetic_input = synthetic_half::document();
    let mut unpinned = seed_document(&synthetic_input, "left");
    add_two_splitters(&mut unpinned, "left", "left");
    unpinned["modules"][0]
        .as_object_mut()
        .unwrap()
        .remove("hostInstanceId");
    let (mut engine, seed) = open(&unpinned);
    let reply = create(
        &mut engine,
        &seed,
        "pair-reuse",
        "left",
        json!({"kind":"reuse-board","instanceId":"right"}),
        mapping(0.0),
    )
    .unwrap();
    let document = &reply["document"];
    assert_eq!(document["modules"], seed["modules"]);
    assert!(module_pairs(document, "pair-reuse").is_empty());

    let mut pinned = seed_document(&synthetic_input, "left");
    add_two_splitters(&mut pinned, "left", "left");
    let (mut engine, seed) = open(&pinned);
    let error = refused(
        &mut engine,
        &seed,
        operation(
            "pair-reuse",
            "left",
            json!({"kind":"reuse-board","instanceId":"right"}),
            mapping(0.0),
        ),
    );
    assert!(
        error.contains("left/splitter-above") && error.contains("reuse-board"),
        "{error}"
    );
}

/// A mirrored module is one Warning per module, located on the mirror, asking a person to confirm
/// its handedness. A source module, or a module that is not part of a pair, raises none.
#[test]
fn every_mirrored_module_raises_one_handed_module_warning_and_a_source_module_raises_none() {
    let synthetic_input = synthetic_half::document();
    let mut source = seed_document(&synthetic_input, "left");
    add_two_splitters(&mut source, "left", "left");
    let opened: Value = serde_json::from_str(
        &CoreEngine::new().request(&json!({"id":"o","kind":"open","document":source}).to_string()),
    )
    .unwrap();
    let handed = |scene: &Value| -> Vec<Value> {
        scene["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["id"].as_str().unwrap().ends_with("/handed-module"))
            .cloned()
            .collect()
    };
    assert!(
        handed(&opened["scene"]).is_empty(),
        "a source module alone is not a finding"
    );

    let (mut engine, seed) = open(&source);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap();
    let findings = handed(&reply["scene"]);
    let mut ids: Vec<&str> = findings.iter().map(|f| f["id"].as_str().unwrap()).collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        [
            "mirror/right/splitter-above/handed-module",
            "mirror/right/splitter-below/handed-module",
        ]
    );
    for finding in &findings {
        assert_eq!(finding["severity"], "warning");
        assert_eq!(finding["scope"], "pcb");
        let target = finding["targetIds"].as_array().unwrap();
        assert_eq!(target.len(), 1);
        assert!(target[0].as_str().unwrap().starts_with("right/"));
    }
}

#[test]
fn handed_off_grid_footprint_warning_is_unique_located_and_pair_scoped() {
    let mut source = synthetic_half::document();
    source["definitions"].as_array_mut().unwrap().push(json!({
        "id":"synthetic/handed-connector", "name":"Handed connector", "kind":"connector",
        "courtyard":[{"x":-3,"y":-2},{"x":4,"y":-2},{"x":4,"y":2},{"x":-3,"y":2}],
        "pads":[{"id":"1","number":"1","at":{"x":-2,"y":0},"size":{"x":1,"y":1},"shape":"circle"}],
        "generator":{"source":"synthetic/handed-connector","version":"1","parameters":{}}
    }));
    source["parts"].as_array_mut().unwrap().push(json!({
        "id":"left/JH1", "definitionId":"synthetic/handed-connector", "reference":"JH1",
        "pose":{"at":{"x":-30,"y":20},"rotation":17}, "side":"front"
    }));
    source["boards"][0]["partIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("left/JH1"));
    let opened: Value = serde_json::from_str(
        &CoreEngine::new()
            .request(&json!({"id":"open","kind":"open","document":source}).to_string()),
    )
    .unwrap();
    let findings = |scene: &Value| -> Vec<Value> {
        scene["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|finding| finding["id"] == "mirror/right/JH1/heading-uncertain")
            .cloned()
            .collect()
    };
    assert!(
        findings(&opened["scene"]).is_empty(),
        "no pair means no mirror heading warning"
    );
    let (mut engine, seed) = open(&opened["document"]);
    let reply = create(
        &mut engine,
        &seed,
        "pair-handed",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let warning = findings(&reply["scene"]);
    assert_eq!(warning.len(), 1);
    assert_eq!(warning[0]["severity"], "warning");
    assert_eq!(warning[0]["scope"], "pcb");
    assert_eq!(warning[0]["targetIds"], json!(["right/JH1"]));
    assert!(
        !reply["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["id"] == "mirror/right/RST/heading-uncertain"),
        "the symmetric synthetic switch is exempt"
    );
}

#[test]
fn each_handed_mirror_has_one_warning_and_symmetric_footprints_have_none() {
    let mut source = synthetic_half::document();
    source["definitions"].as_array_mut().unwrap().push(json!({
        "id":"synthetic/handed-module", "name":"Handed module", "kind":"connector",
        "courtyard":[{"x":-2,"y":-1},{"x":3,"y":-1},{"x":3,"y":1},{"x":-2,"y":1}],
        "pads":[{"id":"1","number":"1","at":{"x":-1,"y":0},"size":{"x":1,"y":1},"shape":"circle"}],
        "generator":{"source":"synthetic/handed-module","version":"1","parameters":{}}
    }));
    for (id, y) in [("left/JH1", 15.0), ("left/JH2", 24.0)] {
        source["parts"].as_array_mut().unwrap().push(json!({
            "id":id, "definitionId":"synthetic/handed-module", "reference":id,
            "pose":{"at":{"x":-30,"y":y},"rotation":17}, "side":"front"
        }));
        source["boards"][0]["partIds"]
            .as_array_mut()
            .unwrap()
            .push(json!(id));
    }
    let (mut engine, seed) = open(&source);
    let reply = create(
        &mut engine,
        &seed,
        "pair-two-handed",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let mut warnings: Vec<_> = reply["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| {
            finding["id"]
                .as_str()
                .unwrap()
                .ends_with("/heading-uncertain")
        })
        .cloned()
        .collect();
    warnings.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    assert_eq!(
        warnings
            .iter()
            .map(|finding| finding["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "mirror/right/JH1/heading-uncertain",
            "mirror/right/JH2/heading-uncertain"
        ]
    );
    for (warning, target) in warnings.iter().zip(["right/JH1", "right/JH2"]) {
        assert_eq!(warning["severity"], "warning");
        assert_eq!(warning["scope"], "pcb");
        assert_eq!(warning["targetIds"], json!([target]));
    }
    assert!(
        !warnings
            .iter()
            .any(|warning| warning["id"] == "mirror/right/RST/heading-uncertain")
    );
}

fn find_part<'a>(document: &'a Value, id: &str) -> Option<&'a Value> {
    document["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
}

/// Sets the turn of matrix cells `(row, column, degrees)` through `set-matrix`, the operation
/// `layout::sync` writes through, so every member part is projected again from the matrix.
fn with_cell_turns(
    engine: &mut CoreEngine,
    document: &Value,
    matrix: &str,
    turns: &[(u64, u64, f64)],
) -> Value {
    let mut matrix_json = document["matrices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == matrix)
        .unwrap()
        .clone();
    for (row, column, degrees) in turns {
        let cell = matrix_json["cells"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["row"] == *row && c["column"] == *column)
            .unwrap_or_else(|| panic!("{matrix}: no cell r{row}c{column}"));
        cell["rotation"] = json!(degrees);
    }
    create_with(
        engine,
        document,
        "commit",
        json!({"kind": "set-matrix", "matrix": matrix_json}),
    )
    .unwrap()["document"]
        .clone()
}

/// A part's position and heading under the pair map `at`, `rotation`: the rule of
/// `assert_reflects_every_part`, returned so a test can compare one part at a time.
fn mapped_pose(map: &Value, pose: &Value) -> ((f64, f64), f64) {
    let (at_x, at_y) = (
        map["at"]["x"].as_f64().unwrap(),
        map["at"]["y"].as_f64().unwrap(),
    );
    let turn = map["rotation"].as_f64().unwrap();
    let (sin, cos) = turn.to_radians().sin_cos();
    let (x, y) = (
        pose["at"]["x"].as_f64().unwrap(),
        pose["at"]["y"].as_f64().unwrap(),
    );
    (
        (-x * cos - y * sin + at_x, -x * sin + y * cos + at_y),
        turn - pose["rotation"].as_f64().unwrap(),
    )
}

/// A companion's position in its switch's frame: the offset turned back by the switch's tilt.
fn in_switch_frame(companion: &Value, switch: &Value) -> (f64, f64) {
    let at = |part: &Value| {
        (
            part["pose"]["at"]["x"].as_f64().unwrap(),
            part["pose"]["at"]["y"].as_f64().unwrap(),
        )
    };
    let ((px, py), (sx, sy)) = (at(companion), at(switch));
    let (sin, cos) = switch["pose"]["rotation"]
        .as_f64()
        .unwrap()
        .to_radians()
        .sin_cos();
    let (dx, dy) = (px - sx, py - sy);
    (dx * cos + dy * sin, -dx * sin + dy * cos)
}

fn assert_close_pair(actual: (f64, f64), expected: (f64, f64), label: &str) {
    assert!(
        (actual.0 - expected.0).abs() < 1e-9 && (actual.1 - expected.1).abs() < 1e-9,
        "{label}: at {actual:?}, expected {expected:?}"
    );
}

/// A key switch takes the pair's heading rule with no half turn: a switch tilted by `r` mirrors to
/// `forward_angle(map, r)`, which is `-r` on an axis map (the two rules differ whenever the map
/// rotation is not 0 modulo 360, which the 27 degree cases cover). A companion is mirrored in
/// position and takes the negated heading: in the switch's own frame its local x flips and its
/// local y does not. A `180 - r` rule would move the LED to the other side, which this test catches.
///
/// Literal-mirror is the default: the diode's authored source-local offset becomes its reflected
/// offset. Duplicate-position keeps the original local offset;
/// that optional mode is tested separately below.
#[test]
fn key_switches_mirror_their_tilt_without_a_half_turn_and_keep_their_companion_side() {
    let cases: [(&str, Option<f64>); 2] = [("synthetic", None), ("synthetic", Some(27.0))];
    for (name, turn) in cases {
        let synthetic_input = &synthetic_half::document();
        let (mut engine, seed) = seed(synthetic_input, "left");
        // Two switches carry a 15 and a -20 degree cell turn (the other two stay flat), so the
        // tilts are not the zero the synthetic_input matrices use.
        let tilted = with_cell_turns(
            &mut engine,
            &seed,
            "left-synthetic-keys",
            &[(0, 0, 15.0), (1, 0, 0.0), (0, 1, -20.0), (1, 1, 0.0)],
        );
        let map = match turn {
            None => mapping(300.0),
            Some(degrees) => json!({"at": {"x": 310.0, "y": 42.0}, "rotation": degrees}),
        };
        let label = format!("{name} with map rotation {:?}", turn.unwrap_or(0.0));
        let reply = create(
            &mut engine,
            &tilted,
            "pair-tilt",
            "left",
            new_board("right", "right"),
            map.clone(),
        )
        .unwrap();
        let mirrored = &reply["document"];
        let source =
            |r: u64, c: u64| part(&tilted, &format!("matrix/left-synthetic-keys/r{r}c{c}"));
        let mirror =
            |r: u64, c: u64| part(mirrored, &format!("matrix/right-synthetic-keys/r{r}c{c}"));
        let tilt = |r: u64, c: u64| source(r, c)["pose"]["rotation"].as_f64().unwrap();
        assert!(
            angles_match(tilt(0, 0) - tilt(1, 0), 15.0)
                && angles_match(tilt(0, 1) - tilt(1, 1), -20.0),
            "{label}: the synthetic source switches must carry the 15 and -20 degree turns"
        );
        for (r, c) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (position, heading) = mapped_pose(&map, &source(r, c)["pose"]);
            assert_close_pair(
                (
                    mirror(r, c)["pose"]["at"]["x"].as_f64().unwrap(),
                    mirror(r, c)["pose"]["at"]["y"].as_f64().unwrap(),
                ),
                position,
                &format!("{label} switch r{r}c{c} position"),
            );
            assert!(
                angles_match(mirror(r, c)["pose"]["rotation"].as_f64().unwrap(), heading),
                "{label} switch r{r}c{c}: heading {} from the pair rule {heading}",
                mirror(r, c)["pose"]["rotation"]
            );
            // The authored diode accompanies every synthetic switch.
            for (suffix, local) in [("diode", (8.0, -2.0))] {
                let source_id = format!("matrix/left-synthetic-keys/r{r}c{c}/{suffix}");
                let Some(companion) = find_part(&tilted, &source_id) else {
                    continue;
                };
                let mirror_id = format!("matrix/right-synthetic-keys/r{r}c{c}/{suffix}");
                let mirror_companion = find_part(mirrored, &mirror_id)
                    .unwrap_or_else(|| panic!("{label}: no mirror part {mirror_id}"));
                let (lx, ly) = in_switch_frame(companion, source(r, c));
                assert!(
                    (lx - local.0).abs() < 1e-6 && (ly - local.1).abs() < 1e-6,
                    "{label} {source_id}: the synthetic component offset changed to ({lx}, {ly})"
                );
                let (position, heading) = mapped_pose(&map, &companion["pose"]);
                assert_close_pair(
                    (
                        mirror_companion["pose"]["at"]["x"].as_f64().unwrap(),
                        mirror_companion["pose"]["at"]["y"].as_f64().unwrap(),
                    ),
                    position,
                    &format!("{label} {mirror_id} position"),
                );
                assert!(
                    angles_match(
                        mirror_companion["pose"]["rotation"].as_f64().unwrap(),
                        heading
                    ),
                    "{label} {mirror_id}: heading from the pair rule {heading}"
                );
                let (mx, my) = in_switch_frame(mirror_companion, mirror(r, c));
                assert!(
                    (mx + local.0).abs() < 1e-6 && (my - local.1).abs() < 1e-6,
                    "{label} {mirror_id}: local ({mx}, {my}) is not the mirrored side ({}, {})",
                    -local.0,
                    local.1
                );
            }
        }
    }
}

/// Each part in the authored synthetic half maps through the pair rule, including companions.
#[test]
fn anchored_geometry_lands_on_the_mapped_world_point_at_every_heading_and_map() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, mut document) = seed(&synthetic_input, "left");
    document = outline_edit(
        &mut engine,
        &document,
        json!({"kind": "board", "boardId": "left"}),
        json!({"copy": {"version_id": "fixed", "name": "Fixed"}}),
        3,
    );
    let companion = "left/TRRS";
    let anchors = [
        ("left/U1", true),
        ("left/RST", true),
        ("matrix/left-synthetic-keys/r0c0", false),
        (companion, false),
    ];
    for map in [
        json!({"at": {"x": 300.0, "y": 0.0}, "rotation": 0.0}),
        json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0}),
    ] {
        for heading in [0.0, 90.0, 180.0, 270.0, 37.0] {
            let mut source = document.clone();
            for part in source["parts"].as_array_mut().unwrap() {
                if part["id"] == "left/U1" || part["id"] == "left/RST" {
                    part["pose"]["rotation"] = json!(heading);
                }
            }
            let features = source["boardOutlines"][0]["versions"][0]["geometry"]["features"]
                .as_array_mut()
                .unwrap();
            for (index, (anchor, _)) in anchors.iter().enumerate() {
                features.push(json!({"kind": "rect", "id": format!("cut-{index}"),
                    "anchorPartId": anchor, "center": {"x": 2.0, "y": 3.5},
                    "size": {"x": 4.0, "y": 5.0}, "radius": 0.0, "rotation": 20.0,
                    "operation": "subtract"}));
            }
            let (mut engine, seed_doc) = open(&source);
            let out = create(
                &mut engine,
                &seed_doc,
                "pair-right",
                "left",
                new_board("right", "right"),
                map.clone(),
            )
            .unwrap();
            let out = &out["document"];
            let state = out["boardOutlines"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["boardId"] == "right")
                .unwrap();
            let features = &state["versions"][0]["geometry"]["features"];
            for (index, (anchor, independent)) in anchors.iter().enumerate() {
                let rect = features
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|f| f["id"] == format!("right/cut-{index}"))
                    .unwrap();
                let mirror_id = rect["anchorPartId"].as_str().unwrap();
                let at = json!({"x": 2.0, "y": 3.5});
                assert_point(
                    world_of(out, mirror_id, &rect["center"]),
                    reflected_point(&map, world_of(out, anchor, &at)),
                    &format!("{anchor} heading {heading} map {}", map["rotation"]),
                );
                if *independent {
                    // The full rule, read independently: `rotation + 180 - heading`.
                    let expected = map["rotation"].as_f64().unwrap() + 180.0 - heading;
                    assert!(
                        angles_match(
                            part(out, mirror_id)["pose"]["rotation"].as_f64().unwrap(),
                            expected
                        ),
                        "{anchor} heading {heading}: {}",
                        part(out, mirror_id)["pose"]["rotation"]
                    );
                }
            }
        }
    }
}

/// Duplicate placement retains the authored switch-local pose under either pair-map rotation.
#[test]
fn duplicate_companions_keep_the_source_local_pose() {
    for turn in [0.0, 27.0] {
        let source_fixture = synthetic_half::document();
        let (mut engine, source) = seed(&source_fixture, "left");
        let source = with_cell_turns(
            &mut engine,
            &source,
            "left-synthetic-keys",
            &[(0, 0, 15.0), (0, 1, -20.0)],
        );
        let mut op = operation(
            "pair-mode",
            "left",
            new_board("right", "right"),
            json!({"at":{"x":310.0,"y":42.0},"rotation":turn}),
        );
        op["companionMode"] = json!("duplicate-position");
        let made = create_with(&mut engine, &source, "commit", op).unwrap()["document"].clone();
        assert_eq!(
            pair(&made, "pair-mode")["halves"]["companionMode"],
            "duplicate-position"
        );
        let mut count = 0;
        for link in pair(&made, "pair-mode")["halves"]["correspondences"]
            .as_array()
            .unwrap()
        {
            if link["kind"] != "part" {
                continue;
            }
            let source_id = link["sourceId"].as_str().unwrap();
            if !source_id.ends_with("/diode") {
                continue;
            }
            let source_key_id = source_id.rsplit_once('/').unwrap().0;
            let mirror_id = link["mirrorId"].as_str().unwrap();
            let mirror_key_id = mirror_id.rsplit_once('/').unwrap().0;
            let original = part(&source, source_id);
            let original_key = part(&source, source_key_id);
            let companion = part(&made, mirror_id);
            let key = part(&made, mirror_key_id);
            assert_close_pair(
                in_switch_frame(original, original_key),
                (8.0, -2.0),
                source_id,
            );
            assert_close_pair(in_switch_frame(companion, key), (8.0, -2.0), mirror_id);
            let relative = original["pose"]["rotation"].as_f64().unwrap()
                - original_key["pose"]["rotation"].as_f64().unwrap();
            let actual = companion["pose"]["rotation"].as_f64().unwrap()
                - key["pose"]["rotation"].as_f64().unwrap();
            assert!(
                (actual - relative).abs() < 1e-9,
                "{mirror_id}: {actual} != {relative}"
            );
            assert_eq!(
                companion["generatorParameters"],
                original["generatorParameters"]
            );
            assert_eq!(companion["side"], original["side"]);
            count += 1;
        }
        assert_eq!(count, 4, "one diode for each authored switch");
        let mut undone =
            request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
        undone["revision"] = source["revision"].clone();
        assert_eq!(undone, source);
        let (_, reopened) = open(&made);
        assert_eq!(pair(&reopened, "pair-mode"), pair(&made, "pair-mode"));
    }
}

#[test]
fn invalid_companion_modes_and_tweaks_are_refused_without_mutation() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let tweak = json!({"x":1.0,"y":2.0,"rotation":3.0});
    for mode in ["literal-mirror", "duplicate-position", "not-a-mode"] {
        let mut op = operation(
            "pair-mode",
            "left",
            new_board("right", "right"),
            mapping(300.0),
        );
        op["companionMode"] = json!(mode);
        op["companionTweak"] = tweak.clone();
        let error = refused(&mut engine, &source, op);
        assert!(
            error.contains("companion") || error.contains("unknown variant"),
            "{error}"
        );
    }
    let made = create(
        &mut engine,
        &source,
        "pair-mode",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap()["document"]
        .clone();
    for exception in [false, true] {
        for mode in [
            None,
            Some("literal-mirror"),
            Some("duplicate-position"),
            Some("unknown"),
        ] {
            let mut invalid = made.clone();
            let record = &mut invalid["hardware"]["pairs"][0]["halves"];
            let index = record["correspondences"]
                .as_array()
                .unwrap()
                .iter()
                .position(|c| c["mirrorId"] == "matrix/right-synthetic-keys/r0c0/diode")
                .unwrap();
            let item = if exception {
                let mut item = record["correspondences"]
                    .as_array_mut()
                    .unwrap()
                    .remove(index);
                item.as_object_mut().unwrap().remove("sourceId");
                item["state"] = json!("local");
                record["exceptions"] = json!([item]);
                &mut record["exceptions"][0]
            } else {
                &mut record["correspondences"][index]
            };
            if let Some(mode) = mode {
                item["companionMode"] = json!(mode);
            }
            item["companionTweak"] = tweak.clone();
            let error = refused(
                &mut engine,
                &made,
                json!({"kind":"replace-document","document":invalid}),
            );
            assert!(
                error.contains("companion") || error.contains("unknown variant"),
                "{error}"
            );
        }
    }
    // The typed boundary can carry non-finite floats, even though JSON cannot.
    for field in ["x", "y", "rotation"] {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut doc: ProjectDoc = serde_json::from_value(made.clone()).unwrap();
            let record = doc.hardware.as_mut().unwrap().pairs[0]
                .halves
                .as_mut()
                .unwrap();
            record.companion_mode =
                boardstudio_core::model::MirroredCompanionMode::LiteralMirrorWithTweak;
            let mut tweak = boardstudio_core::model::MirroredCompanionTweak {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            };
            match field {
                "x" => tweak.x = value,
                "y" => tweak.y = value,
                _ => tweak.rotation = value,
            }
            record
                .correspondences
                .iter_mut()
                .find(|c| c.mirror_id.ends_with("/diode"))
                .unwrap()
                .companion_tweak = Some(tweak);
            record
                .correspondences
                .iter_mut()
                .find(|c| c.mirror_id.ends_with("/diode"))
                .unwrap()
                .companion_mode =
                Some(boardstudio_core::model::MirroredCompanionMode::LiteralMirrorWithTweak);
            let error = boardstudio_core::mechanical::halves::validate_document(&doc).unwrap_err();
            assert!(error.contains("finite"), "{error}");
        }
    }
}

#[test]
fn a_plain_led_companion_uses_its_matrix_relationship_in_every_mode() {
    for (mode, offset, heading) in [
        (None, (-2.5, 3.25), -37.0),
        (Some("literal-mirror"), (-2.5, 3.25), -37.0),
        (Some("duplicate-position"), (2.5, 3.25), 37.0),
        (Some("literal-mirror-with-tweak"), (-1.25, 1.25), -24.0),
    ] {
        let (mut engine, source) = seed(&runtime_supported_document(), "left");
        let mut matrix = source["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == "left-synthetic-keys")
            .unwrap()
            .clone();
        let cell = matrix["cells"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["row"] == 0 && c["column"] == 0)
            .unwrap();
        cell["rotation"] = json!(15.0);
        cell["assemblies"].as_array_mut().unwrap().push(json!({"id":"status-light","definitionId":"plain-led","offset":{"x":2.5,"y":3.25},"rotation":37.0,"side":"back"}));
        let source = create_with(&mut engine, &source, "commit", json!({"kind":"set-matrix","matrix":matrix,"definitions":[{"id":"plain-led","name":"Single colour LED","kind":"passive","pads":[],"courtyard":[]}]})).unwrap()["document"].clone();
        let mut op = operation(
            "pair-led",
            "left",
            new_board("right", "right"),
            json!({"at":{"x":310.0,"y":42.0},"rotation":27.0}),
        );
        if let Some(mode) = mode {
            op["companionMode"] = json!(mode);
        }
        if mode == Some("literal-mirror-with-tweak") {
            op["companionTweak"] = json!({"x":1.25,"y":-2.0,"rotation":13.0});
        }
        let made = create_with(&mut engine, &source, "commit", op).unwrap()["document"].clone();
        let led = part(&made, "matrix/right-synthetic-keys/r0c0/status-light");
        let switch = part(&made, "matrix/right-synthetic-keys/r0c0");
        assert_close_pair(in_switch_frame(led, switch), offset, "plain LED");
        let relative = led["pose"]["rotation"].as_f64().unwrap()
            - switch["pose"]["rotation"].as_f64().unwrap();
        assert!(
            (relative - heading).abs() < 1e-9,
            "plain LED in {mode:?}: {relative} != {heading}"
        );
        let assembly = made["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == "right-synthetic-keys")
            .unwrap()["cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["row"] == 0 && c["column"] == 0)
            .unwrap()["assemblies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == "status-light")
            .unwrap();
        assert_close_pair(
            (
                assembly["offset"]["x"].as_f64().unwrap(),
                assembly["offset"]["y"].as_f64().unwrap(),
            ),
            offset,
            "stored plain LED assembly",
        );
        assert!((assembly["rotation"].as_f64().unwrap() - heading).abs() < 1e-9);
        let link = pair(&made, "pair-led")["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["mirrorId"] == led["id"])
            .unwrap();
        assert_eq!(
            link["companionTweak"].is_object(),
            mode == Some("literal-mirror-with-tweak")
        );
    }
}

#[test]
fn companion_overrides_and_tweaks_survive_linked_local_and_omitted_records() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let made = create(
        &mut engine,
        &source,
        "pair-mode",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap()["document"]
        .clone();
    for default in ["duplicate-position", "literal-mirror-with-tweak"] {
        for state in ["linked", "local", "omitted"] {
            let mut document = made.clone();
            let halves = &mut document["hardware"]["pairs"][0]["halves"];
            halves["companionMode"] = json!(default);
            let links = halves["correspondences"].as_array_mut().unwrap();
            let index = links
                .iter()
                .position(|c| c["mirrorId"] == "matrix/right-synthetic-keys/r0c0/diode")
                .unwrap();
            links[index]["companionMode"] = json!("literal-mirror-with-tweak");
            links[index]["companionTweak"] = json!({"x":1.0,"y":2.0,"rotation":3.0});
            let other = links
                .iter_mut()
                .find(|c| c["mirrorId"] == "matrix/right-synthetic-keys/r0c1/diode")
                .unwrap();
            other["companionMode"] = json!("literal-mirror");
            if state != "linked" {
                let mut item = links.remove(index);
                item.as_object_mut().unwrap().remove("sourceId");
                item["state"] = json!(state);
                halves["exceptions"] = json!([item]);
            }
            let (_, reopened) = open(&document);
            assert_eq!(
                pair(&reopened, "pair-mode")["halves"],
                pair(&document, "pair-mode")["halves"]
            );
        }
    }
}

#[test]
fn typed_creation_refuses_non_finite_tweaks_without_mutation() {
    use boardstudio_core::model::{
        CoreReply, CoreRequest, EditCommand, EditOperation, MirroredCompanionTweak,
    };
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    for field in ["x", "y", "rotation"] {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut op = operation(
                "pair-finite",
                "left",
                new_board("right", "right"),
                mapping(300.0),
            );
            op["companionMode"] = json!("literal-mirror-with-tweak");
            let mut command: EditCommand = serde_json::from_value(json!({"baseRevision":source["revision"],"transactionId":"finite","phase":"commit","targetIds":[],"operation":op})).unwrap();
            let mut tweak = MirroredCompanionTweak {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            };
            match field {
                "x" => tweak.x = value,
                "y" => tweak.y = value,
                _ => tweak.rotation = value,
            }
            let EditOperation::CreateMirroredHalf {
                companion_tweak, ..
            } = &mut command.operation
            else {
                panic!("creation operation")
            };
            *companion_tweak = Some(tweak);
            match engine.handle(CoreRequest::Edit {
                id: "finite".into(),
                command,
            }) {
                CoreReply::Error { message, .. } => {
                    assert!(message.contains("finite"), "{message}")
                }
                other => panic!("Non-finite tweak accepted: {other:?}"),
            }
            assert_eq!(snapshot(&mut engine), source);
        }
    }
}

#[test]
fn a_reused_board_cannot_take_a_tweak_that_it_cannot_store_per_item() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let mut op = operation(
        "pair-reuse",
        "left",
        json!({"kind":"reuse-board","instanceId":"flipped"}),
        mapping(0.0),
    );
    op["companionMode"] = json!("literal-mirror-with-tweak");
    op["companionTweak"] = json!({"x":1.0,"y":2.0,"rotation":3.0});
    let error = refused(&mut engine, &source, op);
    assert!(
        error.contains("reuse-board") && error.contains("companionTweak"),
        "{error}"
    );
}

/// Labels, preview/commit behavior, and undo/redo follow the mirrored-half operation contract.
fn unibody_label_seed() -> (CoreEngine, Value) {
    let mut document = synthetic_half::document();
    let source = &mut document["hardware"]["instances"][0];
    source["half"] = json!("unibody");
    source["name"] = json!("Keyboard");
    source["role"] = json!("standalone");
    open(&document)
}

fn split_labels() -> Value {
    json!({
        "source": {"name":"Left half", "half":"left", "role":"central"},
        "mirror": {"name":"Right half", "half":"right", "role":"peripheral"}
    })
}

#[test]
fn creation_labels_apply_atomically_in_both_modes_and_undo_redo_together() {
    for mode in [
        new_board("right", "right"),
        json!({"kind":"reuse-board","instanceId":"right"}),
    ] {
        let (mut engine, before) = unibody_label_seed();
        let mut op = operation("pair-labels", "left", mode, mapping(0.0));
        op["labels"] = split_labels();
        let preview = create_with(&mut engine, &before, "preview", op.clone()).unwrap();
        assert_eq!(preview["kind"], "preview");
        assert_eq!(
            snapshot(&mut engine),
            before,
            "preview must not mutate the accepted source"
        );
        let reply = create_with(&mut engine, &before, "commit", op).unwrap();
        let made = &reply["document"];
        for (id, side) in [("left", "source"), ("right", "mirror")] {
            for field in ["name", "half", "role"] {
                assert_eq!(
                    instance(made, id)[field],
                    split_labels()[side][field],
                    "{id}.{field}"
                );
            }
            assert!(
                reply["scene"]["changedIds"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(id))
            );
        }
        assert_eq!(
            made["revision"].as_u64().unwrap(),
            before["revision"].as_u64().unwrap() + 1
        );
        let (_, reopened) = open(made);
        assert_eq!(
            reopened["hardware"]["instances"],
            made["hardware"]["instances"]
        );
        let mut undone =
            request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
        undone["revision"] = before["revision"].clone();
        assert_eq!(undone, before);
        let mut redone =
            request(&mut engine, json!({"id":"redo","kind":"redo"}))["document"].clone();
        redone["revision"] = made["revision"].clone();
        assert_eq!(&redone, made);
    }
}

#[test]
fn omitted_and_empty_creation_labels_preserve_unibody_defaults() {
    for mode in [
        new_board("right", "right"),
        json!({"kind":"reuse-board","instanceId":"right"}),
    ] {
        for labels in [
            None,
            Some(json!({})),
            Some(json!({"source":{},"mirror":{}})),
        ] {
            let (mut engine, before) = unibody_label_seed();
            let mut op = operation("pair-labels", "left", mode.clone(), mapping(0.0));
            if let Some(labels) = labels {
                op["labels"] = labels;
            }
            let made = create_with(&mut engine, &before, "commit", op).unwrap()["document"].clone();
            for (id, name, role) in [
                ("left", "Keyboard", "standalone"),
                ("right", "Keyboard mirror", "central"),
            ] {
                let half = instance(&made, id);
                assert_eq!(half["name"], name);
                assert_eq!(half["half"], "unibody");
                assert_eq!(half["role"], role);
            }
        }
    }
}

#[test]
fn partial_creation_labels_only_replace_the_supplied_fields() {
    for labels in [
        json!({"source":{"name":"Primary"}}),
        json!({"mirror":{"half":"right"}}),
        json!({"source":{"half":"right"},"mirror":{"role":"standalone"}}),
        json!({"source":{"role":"central"},"mirror":{"half":"right"}}),
    ] {
        let (mut engine, before) = unibody_label_seed();
        let mut op = operation(
            "pair-labels",
            "left",
            new_board("right", "right"),
            mapping(0.0),
        );
        op["labels"] = labels.clone();
        let made = create_with(&mut engine, &before, "commit", op).unwrap()["document"].clone();
        for (id, side, name, role) in [
            ("left", "source", "Keyboard", "standalone"),
            ("right", "mirror", "Keyboard mirror", "central"),
        ] {
            for (field, default) in [("name", name), ("half", "unibody"), ("role", role)] {
                assert_eq!(
                    instance(&made, id)[field],
                    labels[side].get(field).cloned().unwrap_or(json!(default)),
                    "{id}.{field}"
                );
            }
        }
    }
}

#[test]
fn invalid_creation_labels_are_located_and_refused_without_mutation() {
    for mode in [
        new_board("right", "right"),
        json!({"kind":"reuse-board","instanceId":"right"}),
    ] {
        for (labels, location) in [
            (json!({"source":{"name":" \t"}}), "labels.source.name"),
            (json!({"mirror":{"name":""}}), "labels.mirror.name"),
            (json!({"source":{"half":"middle"}}), "labels.source.half"),
            (json!({"mirror":{"half":""}}), "labels.mirror.half"),
            (json!({"source":{"role":"primary"}}), "labels.source.role"),
            (json!({"mirror":{"role":"unknown"}}), "labels.mirror.role"),
            (
                json!({"source":{"half":"left","role":"central"},"mirror":{"half":"left","role":"central"}}),
                "labels.mirror",
            ),
            (json!({"source":{"role":"central"}}), "labels.mirror"),
            (json!({"mirror":{"role":"standalone"}}), "labels.mirror"),
        ] {
            let (mut engine, before) = unibody_label_seed();
            let mut op = operation("pair-labels", "left", mode.clone(), mapping(0.0));
            op["labels"] = labels;
            let error = refused(&mut engine, &before, op);
            assert!(error.contains(location), "{location}: {error}");
        }
    }
}

#[test]
fn stabilisers_keep_their_source_heading_and_mirrored_position_in_every_mode() {
    let imported: Value =
        serde_json::from_str(include_str!("../../catalogue/parts/imported-parts.json")).unwrap();
    let definitions: Vec<Value> = imported["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["definition"].clone())
        .filter(|definition| {
            [
                "kicad:marbastlib/STAB_MX_2u",
                "kicad:marbastlib/STAB_MX_6.25u",
            ]
            .contains(&definition["id"].as_str().unwrap())
        })
        .collect();
    assert_eq!(definitions.len(), 2);
    for relative_heading in [0.0, 90.0, 15.0] {
        let (mut engine, source) = seed(&runtime_supported_document(), "left");
        let mut matrix = source["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|matrix| matrix["id"] == "left-synthetic-keys")
            .unwrap()
            .clone();
        let cell = matrix["cells"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|cell| cell["row"] == 0 && cell["column"] == 0)
            .unwrap();
        cell["rotation"] = json!(15.0);
        for (index, definition) in definitions.iter().enumerate() {
            cell["assemblies"].as_array_mut().unwrap().push(json!({
                "id":format!("stabiliser-{index}"), "definitionId":definition["id"],
                "offset":{"x":2.5,"y":3.25}, "rotation":relative_heading, "side":"back"
            }));
        }
        let source = create_with(
            &mut engine,
            &source,
            "commit",
            json!({"kind":"set-matrix","matrix":matrix,"definitions":definitions}),
        )
        .unwrap()["document"]
            .clone();
        for turn in [0.0, 27.0] {
            for mode in [
                "literal-mirror",
                "duplicate-position",
                "literal-mirror-with-tweak",
            ] {
                let (mut engine, source) = open(&source);
                let map = json!({"at":{"x":310.0,"y":42.0},"rotation":turn});
                let mut op = operation(
                    "pair-stabilisers",
                    "left",
                    new_board("right", "right"),
                    map.clone(),
                );
                op["companionMode"] = json!(mode);
                if mode == "literal-mirror-with-tweak" {
                    op["companionTweak"] = json!({"x":1.25,"y":-2.0,"rotation":13.0});
                }
                let made =
                    create_with(&mut engine, &source, "commit", op).unwrap()["document"].clone();
                for index in 0..definitions.len() {
                    let source_id = format!("matrix/left-synthetic-keys/r0c0/stabiliser-{index}");
                    let mirror_id = format!("matrix/right-synthetic-keys/r0c0/stabiliser-{index}");
                    let original = part(&source, &source_id);
                    let mirrored = part(&made, &mirror_id);
                    let label =
                        format!("{mode} map {turn} relative {relative_heading}: {mirror_id}");
                    assert_close_pair(
                        (
                            mirrored["pose"]["at"]["x"].as_f64().unwrap(),
                            mirrored["pose"]["at"]["y"].as_f64().unwrap(),
                        ),
                        mapped_pose(&map, &original["pose"]).0,
                        &label,
                    );
                    assert_eq!(
                        mirrored["pose"]["rotation"], original["pose"]["rotation"],
                        "{label}"
                    );
                    for field in ["definitionId", "side", "generatorParameters"] {
                        assert_eq!(mirrored[field], original[field], "{label}: {field}");
                    }
                }
                let (_, reopened) = open(&made);
                assert_eq!(reopened["matrices"], made["matrices"]);
                assert_eq!(reopened["parts"], made["parts"]);
            }
        }
    }
}

#[test]
fn missing_source_companions_materialise_with_a_literal_fallback_in_modes_two_and_three() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let intact = with_cell_turns(&mut engine, &source, "left-synthetic-keys", &[(0, 0, 15.0)]);
    let ids = [
        "matrix/left-synthetic-keys/r0c0/diode",
        "matrix/left-synthetic-keys/r0c0/led",
    ];
    for listed in [true, false] {
        let mut source = intact.clone();
        source["parts"]
            .as_array_mut()
            .unwrap()
            .retain(|part| !ids.contains(&part["id"].as_str().unwrap()));
        if !listed {
            for board in source["boards"].as_array_mut().unwrap() {
                board["partIds"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|id| !ids.contains(&id.as_str().unwrap()));
            }
            for matrix in source["matrices"].as_array_mut().unwrap() {
                matrix["partIds"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|id| !ids.contains(&id.as_str().unwrap()));
            }
        }
        for turn in [0.0, 27.0] {
            for mode in ["duplicate-position", "literal-mirror-with-tweak"] {
                let (mut engine, before) = open(&source);
                assert!(ids.iter().all(|id| find_part(&before, id).is_none()));
                let map = json!({"at":{"x":310.0,"y":42.0},"rotation":turn});
                let mut op = operation(
                    "pair-fallback",
                    "left",
                    new_board("right", "right"),
                    map.clone(),
                );
                op["companionMode"] = json!(mode);
                if mode == "literal-mirror-with-tweak" {
                    op["companionTweak"] = json!({"x":1.25,"y":-2.0,"rotation":13.0});
                }
                let made =
                    create_with(&mut engine, &before, "commit", op).unwrap()["document"].clone();
                for id in ids {
                    let mirror_id = id.replace("left-synthetic-keys", "right-synthetic-keys");
                    let original = part(&intact, id);
                    let mirrored = part(&made, &mirror_id);
                    let expected = mapped_pose(&map, &original["pose"]);
                    let label = format!("{mode} map {turn} listed {listed}: {mirror_id}");
                    assert_close_pair(
                        (
                            mirrored["pose"]["at"]["x"].as_f64().unwrap(),
                            mirrored["pose"]["at"]["y"].as_f64().unwrap(),
                        ),
                        expected.0,
                        &label,
                    );
                    assert!(
                        (mirrored["pose"]["rotation"].as_f64().unwrap() - expected.1).abs() < 1e-9,
                        "{label}"
                    );
                    // The source is repaired too, so both sides have a real, owned part to link.
                    assert_eq!(
                        part(&made, id)["pose"],
                        original["pose"],
                        "{label}: source pose"
                    );
                    let link = pair(&made, "pair-fallback")["halves"]["correspondences"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|link| link["mirrorId"] == mirror_id)
                        .unwrap();
                    assert_eq!(
                        link["companionMode"], "literal-mirror",
                        "{label}: durable fallback"
                    );
                    assert!(
                        link.get("companionTweak").is_none(),
                        "{label}: fallback ignores the tweak"
                    );
                }
                for matrix in made["matrices"].as_array().unwrap() {
                    let board = made["boards"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|board| board["id"] == matrix["boardId"])
                        .unwrap();
                    for id in matrix["partIds"].as_array().unwrap() {
                        assert!(
                            find_part(&made, id.as_str().unwrap()).is_some(),
                            "unmaterialised {id}"
                        );
                        assert!(
                            board["partIds"].as_array().unwrap().contains(id),
                            "unowned {id}"
                        );
                    }
                }
                for original in before["parts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|part| part["id"].as_str().unwrap().starts_with("matrix/"))
                {
                    assert_eq!(part(&made, original["id"].as_str().unwrap()), original);
                }
                let (_, reopened) = open(&made);
                assert_eq!(reopened["parts"], made["parts"]);
                assert_eq!(
                    pair(&reopened, "pair-fallback"),
                    pair(&made, "pair-fallback")
                );
                let mut undone =
                    request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
                undone["revision"] = before["revision"].clone();
                assert_eq!(undone, before);
            }
        }
    }
}

#[test]
fn a_missing_switch_is_refused_without_mutation_even_with_missing_companions() {
    let mut source = seed_document(&synthetic_half::document(), "left");
    let key_id = "matrix/left-synthetic-keys/r0c0";
    let belongs_to_cell = |id: &str| id == key_id || id.starts_with(&format!("{key_id}/"));
    source["parts"]
        .as_array_mut()
        .unwrap()
        .retain(|part| !belongs_to_cell(part["id"].as_str().unwrap()));
    for board in source["boards"].as_array_mut().unwrap() {
        board["partIds"]
            .as_array_mut()
            .unwrap()
            .retain(|id| !belongs_to_cell(id.as_str().unwrap()));
    }
    for matrix in source["matrices"].as_array_mut().unwrap() {
        matrix["partIds"]
            .as_array_mut()
            .unwrap()
            .retain(|id| !belongs_to_cell(id.as_str().unwrap()));
    }
    for mode in [
        "literal-mirror",
        "duplicate-position",
        "literal-mirror-with-tweak",
    ] {
        let (mut engine, before) = open(&source);
        assert!(find_part(&before, key_id).is_none());
        let mut op = operation(
            "pair-missing-switch",
            "left",
            new_board("right", "right"),
            mapping(310.0),
        );
        op["companionMode"] = json!(mode);
        if mode == "literal-mirror-with-tweak" {
            op["companionTweak"] = json!({"x":1.25,"y":-2.0,"rotation":13.0});
        }
        let error = refused(&mut engine, &before, op);
        assert!(
            error.contains("source switch") && error.contains(key_id),
            "{mode}: {error}"
        );
    }
}

#[test]
fn production_prepared_routes_mirror_edits_preview_and_history() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let original = seed.clone();
    let created = create(
        &mut engine,
        &seed,
        "runtime-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let accepted = created["document"].clone();
    let move_from_mirror = json!({
        "kind":"move-parts",
        "positions":[{"id":"right/U1","at":{"x":216.0,"y":14.0}}]
    });
    let preview = request(
        &mut engine,
        json!({"id":"runtime-preview","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"runtime-preview",
            "phase":"preview",
            "targetIds":["right/U1"],
            "operation":move_from_mirror
        }}),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    let preview_transform = |id: &str| {
        preview["scene"]["transforms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|transform| transform["id"] == id)
            .unwrap_or_else(|| panic!("preview transform for {id} is missing: {preview}"))
    };
    assert_eq!(preview_transform("left/U1")["pose"]["at"]["x"], -16.0);
    assert_eq!(preview_transform("left/U1")["pose"]["at"]["y"], 14.0);
    assert_eq!(preview_transform("right/U1")["pose"]["at"]["x"], 216.0);
    assert_eq!(preview_transform("right/U1")["pose"]["at"]["y"], 14.0);
    assert_eq!(snapshot(&mut engine), accepted, "preview must not publish");

    let committed = request(
        &mut engine,
        json!({"id":"runtime-commit","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"runtime-commit",
            "phase":"commit",
            "targetIds":["right/U1"],
            "operation":move_from_mirror
        }}),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    assert_eq!(
        committed["document"]["revision"],
        accepted["revision"].as_u64().unwrap() + 1
    );
    assert_eq!(
        part(&committed["document"], "left/U1")["pose"]["at"]["x"],
        -16.0
    );
    assert_eq!(
        part(&committed["document"], "right/U1")["pose"]["at"]["x"],
        216.0
    );

    let undone = request(&mut engine, json!({"id":"runtime-undo","kind":"undo"}));
    assert_eq!(
        part(&undone["document"], "left/U1")["pose"]["at"]["x"],
        -24.0
    );
    assert_eq!(
        part(&undone["document"], "right/U1")["pose"]["at"]["x"],
        224.0
    );

    let second_undo = request(
        &mut engine,
        json!({"id":"runtime-second-undo","kind":"undo"}),
    );
    assert_eq!(
        second_undo["document"]["hardware"]["pairs"], original["hardware"]["pairs"],
        "one prepared commit must publish exactly one history snapshot"
    );
    let first_redo = request(
        &mut engine,
        json!({"id":"runtime-first-redo","kind":"redo"}),
    );
    assert_eq!(
        pair(&first_redo["document"], "runtime-pair")["id"],
        "runtime-pair"
    );
    let redone = request(&mut engine, json!({"id":"runtime-redo","kind":"redo"}));
    assert_eq!(
        part(&redone["document"], "left/U1")["pose"]["at"]["x"],
        -16.0
    );
    assert_eq!(
        part(&redone["document"], "right/U1")["pose"]["at"]["x"],
        216.0
    );
}

#[test]
fn production_open_repairs_native_pair_and_reports_located_finding() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        "runtime-open-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let mut edited_mirror = created["document"].clone();
    let mirror = edited_mirror["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|part| part["id"] == "right/U1")
        .unwrap();
    mirror["pose"]["at"]["x"] = json!(231.0);
    let mut reopened_engine = CoreEngine::new();
    let reopened = request(
        &mut reopened_engine,
        json!({"id":"runtime-reopen","kind":"open","document":edited_mirror}),
    );
    assert_eq!(reopened["kind"], "scene", "{reopened}");
    assert_eq!(
        part(&reopened["document"], "right/U1")["pose"]["at"]["x"],
        224.0
    );
    assert!(
        reopened["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["id"] == "mirrored-half:open-repaired:runtime-open-pair")
    );
}

#[test]
fn production_prepared_circuit_addition_preserves_member_frames_and_bidirectional_edits() {
    for side in ["front", "back"] {
        let (mut engine, seed) = seed(&runtime_supported_document(), "left");
        let created = create(
            &mut engine,
            &seed,
            &format!("runtime-circuit-{side}"),
            "left",
            new_board("right", "right"),
            mapping(200.0),
        )
        .unwrap();
        let mut definition = imported_haptic();
        let member = &mut definition["circuit"]["parts"][0];
        member["pose"] = json!({"at":{"x":3.75,"y":8.5},"rotation":359.0});
        member["side"] = json!("back");
        let added = request(
            &mut engine,
            json!({"id":"runtime-circuit-add","kind":"edit","command":{
                "baseRevision":created["document"]["revision"],
                "transactionId":"runtime-circuit-add",
                "phase":"commit",
                "targetIds":["right/runtime-circuit"],
                "operation":{
                    "kind":"embed-module-circuit",
                    "id":"right/runtime-circuit",
                    "definition":definition,
                    "hostBoardId":"right",
                    "pose":{"at":{"x":120.0,"y":40.0},"rotation":370.0},
                    "side":side,
                    "joins":{}
                }
            }}),
        );
        assert_eq!(added["kind"], "scene", "{added}");
        let document = &added["document"];
        let circuit_pair = pairs_of_kind(
            document,
            &format!("runtime-circuit-{side}"),
            "embedded-circuit",
        )
        .into_iter()
        .find(|(_, mirror_id)| mirror_id == "right/runtime-circuit")
        .unwrap_or_else(|| panic!("the new circuit pair is absent: {document}"));
        let typed_member = &definition["circuit"]["parts"][0];
        let typed_member_id = typed_member["id"].as_str().unwrap();
        let typed_member_reference = typed_member["reference"].as_str().unwrap();
        let member_candidates: Vec<_> =
            pairs_of_kind(document, &format!("runtime-circuit-{side}"), "part")
                .into_iter()
                .filter(|(source_id, mirror_id)| {
                    source_id.starts_with(&format!("embedded/{}/", circuit_pair.0))
                        && source_id.rsplit('/').next() == Some(typed_member_id)
                        && part(document, source_id)["reference"] == typed_member_reference
                        && mirror_id.starts_with("embedded/right/runtime-circuit/")
                })
                .collect();
        assert_eq!(
            member_candidates.len(),
            1,
            "the typed member must resolve to exactly one source/mirror correspondence: {member_candidates:?}"
        );
        let member_pair = member_candidates.into_iter().next().unwrap();
        let source_member = part(document, &member_pair.0);
        let mirror_member = part(document, &member_pair.1);
        assert_eq!(
            typed_member["side"], "back",
            "the fixture exercises an already-Back member"
        );
        assert_eq!(typed_member["pose"]["rotation"], 359.0);
        let local_x = if side == "back" { -3.75 } else { 3.75 };
        let (sin, cos) = 370.0_f64.to_radians().sin_cos();
        let expected_typed_at = (
            120.0 + local_x * cos - 8.5 * sin,
            40.0 + local_x * sin + 8.5 * cos,
        );
        assert_point(
            (
                mirror_member["pose"]["at"]["x"].as_f64().unwrap(),
                mirror_member["pose"]["at"]["y"].as_f64().unwrap(),
            ),
            expected_typed_at,
            "the mirror circuit member must land at the originally requested typed pose",
        );
        let expected_typed_heading = if side == "back" { 11.0 } else { 729.0 };
        assert!(
            angles_match(
                mirror_member["pose"]["rotation"].as_f64().unwrap(),
                expected_typed_heading,
            ),
            "the typed member heading must wrap from {expected_typed_heading}"
        );
        assert_eq!(
            mirror_member["side"],
            if side == "back" { "front" } else { "back" },
            "the already-Back definition member flips with the host side"
        );
        assert_ne!(
            source_member["pose"]["at"]["y"], 0,
            "the member's local y must participate"
        );
        assert_eq!(source_member["side"], mirror_member["side"]);
        assert_point(
            (
                mirror_member["pose"]["at"]["x"].as_f64().unwrap(),
                mirror_member["pose"]["at"]["y"].as_f64().unwrap(),
            ),
            reflected_point(
                &mapping(200.0),
                (
                    source_member["pose"]["at"]["x"].as_f64().unwrap(),
                    source_member["pose"]["at"]["y"].as_f64().unwrap(),
                ),
            ),
            "new circuit member mirror position",
        );
        assert!(angles_match(
            mirror_member["pose"]["rotation"].as_f64().unwrap(),
            180.0 - source_member["pose"]["rotation"].as_f64().unwrap(),
        ));

        for (source_id, target_id, at) in [
            (&member_pair.1, &member_pair.0, json!({"x":72.0,"y":31.0})),
            (&member_pair.0, &member_pair.1, json!({"x":-32.0,"y":-13.0})),
        ] {
            let before = snapshot(&mut engine);
            let moved = request(
                &mut engine,
                json!({"id":"runtime-circuit-member-move","kind":"edit","command":{
                    "baseRevision":before["revision"],
                    "transactionId":format!("runtime-circuit-member-move-{source_id}"),
                    "phase":"commit",
                    "targetIds":[source_id],
                    "operation":{"kind":"move-parts","positions":[{"id":source_id,"at":at}]}
                }}),
            );
            assert_eq!(moved["kind"], "scene", "{moved}");
            let moved_source = part(&moved["document"], target_id);
            let moved_mirror = part(
                &moved["document"],
                if target_id == &member_pair.0 {
                    &member_pair.1
                } else {
                    &member_pair.0
                },
            );
            assert_point(
                (
                    moved_mirror["pose"]["at"]["x"].as_f64().unwrap(),
                    moved_mirror["pose"]["at"]["y"].as_f64().unwrap(),
                ),
                reflected_point(
                    &mapping(200.0),
                    (
                        moved_source["pose"]["at"]["x"].as_f64().unwrap(),
                        moved_source["pose"]["at"]["y"].as_f64().unwrap(),
                    ),
                ),
                "subsequent circuit-member edit",
            );
        }
    }
}

#[test]
fn production_prepared_routes_commit_exceptions_and_nonempty_unlink_atomically() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        "runtime-lifecycle-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let before_local = created["document"].clone();
    let local = request(
        &mut engine,
        json!({"id":"runtime-local","kind":"edit","command":{
            "baseRevision":before_local["revision"],
            "transactionId":"runtime-local",
            "phase":"commit",
            "targetIds":["right/RST"],
            "operation":{
                "kind":"set-half-exception",
                "pairId":"runtime-lifecycle-pair",
                "itemKind":"part",
                "mirrorId":"right/RST",
                "state":"local"
            }
        }}),
    );
    assert_eq!(local["kind"], "scene", "{local}");
    assert_eq!(
        pair(&local["document"], "runtime-lifecycle-pair")["halves"]["exceptions"][0]["state"],
        "local"
    );

    let unlinked = request(
        &mut engine,
        json!({"id":"runtime-unlink","kind":"edit","command":{
            "baseRevision":local["document"]["revision"],
            "transactionId":"runtime-unlink",
            "phase":"commit",
            "targetIds":[],
            "operation":{"kind":"unlink-halves","pairId":"runtime-lifecycle-pair"}
        }}),
    );
    assert_eq!(unlinked["kind"], "scene", "{unlinked}");
    assert!(
        !unlinked["document"]["hardware"]["pairs"]
            .as_array()
            .is_some_and(|pairs| pairs
                .iter()
                .any(|record| record["id"] == "runtime-lifecycle-pair"))
    );

    let undone = request(
        &mut engine,
        json!({"id":"runtime-unlink-undo","kind":"undo"}),
    );
    assert_eq!(
        pair(&undone["document"], "runtime-lifecycle-pair")["halves"]["exceptions"][0]["state"],
        "local"
    );
    let redone = request(
        &mut engine,
        json!({"id":"runtime-unlink-redo","kind":"redo"}),
    );
    assert!(
        !redone["document"]["hardware"]["pairs"]
            .as_array()
            .is_some_and(|pairs| pairs
                .iter()
                .any(|record| record["id"] == "runtime-lifecycle-pair"))
    );
}

#[test]
fn production_prepared_refusal_keeps_document_revision_and_history_untouched() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    create(
        &mut engine,
        &seed,
        "runtime-refusal-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let undone_creation = request(
        &mut engine,
        json!({"id":"runtime-refusal-undo-first","kind":"undo"}),
    );
    assert_eq!(undone_creation["document"]["hardware"], seed["hardware"]);
    let before = undone_creation["document"].clone();
    let refused: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"runtime-prepared-refusal","kind":"edit","command":{
                "baseRevision":before["revision"],
                "transactionId":"runtime-prepared-refusal",
                "phase":"commit",
                "targetIds":[],
                "operation":{
                    "kind":"set-half-exception",
                    "pairId":"missing-pair",
                    "itemKind":"part",
                    "mirrorId":"right/U1",
                    "state":"local"
                }
            }})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(refused["kind"], "error", "{refused}");
    let after_refusal = snapshot(&mut engine);
    assert_eq!(after_refusal["revision"], before["revision"]);
    assert_eq!(after_refusal, before, "a prepared refusal must not publish");

    let redone = request(
        &mut engine,
        json!({"id":"runtime-refusal-redo","kind":"redo"}),
    );
    assert!(
        redone["document"]["hardware"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["id"] == "runtime-refusal-pair")
    );
}

#[test]
fn production_targeted_instance_set_mechanical_uses_preparation_for_unpaired_instances() {
    for initially_disabled in [false, true] {
        let mut document = runtime_supported_document();
        let mut configuration = document["hardware"]["instances"][0]["mechanical"].clone();
        configuration["wallThickness"] = json!(3.0);
        if initially_disabled {
            document["hardware"]["instances"][0]["mechanical"] = Value::Null;
        }
        let (mut engine, before) = open(&document);
        let configured = request(
            &mut engine,
            json!({"id":"runtime-instance-mechanical-configure","kind":"edit","command":{
                "baseRevision":before["revision"],
                "transactionId":format!("runtime-instance-mechanical-configure-{initially_disabled}"),
                "phase":"commit",
                "targetIds":[],
                "operation":{
                    "kind":"set-mechanical",
                    "configuration":configuration,
                    "target":{"kind":"instance","instanceId":"left"}
                }
            }}),
        );
        assert_eq!(
            configured["kind"], "scene",
            "disabled={initially_disabled}: {configured}"
        );
        assert_eq!(
            instance(&configured["document"], "left")["mechanical"]["wallThickness"],
            3.0
        );
        assert_eq!(
            configured["document"]["revision"],
            before["revision"].as_u64().unwrap() + 1
        );
    }
}

#[test]
fn production_matrix_membership_transition_updates_pair_context_for_later_source_edits() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        "runtime-membership-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let before = created["document"].clone();
    let removed = request(
        &mut engine,
        json!({"id":"runtime-remove-cell","kind":"edit","command":{
            "baseRevision":before["revision"],
            "transactionId":"runtime-remove-cell",
            "phase":"commit",
            "targetIds":[],
            "operation":{
                "kind":"remove-selected-matrix-cells",
                "cells":{"left-synthetic-keys":[[0,0]]}
            }
        }}),
    );
    assert_eq!(removed["kind"], "scene", "{removed}");
    let correspondences =
        pair(&removed["document"], "runtime-membership-pair")["halves"]["correspondences"]
            .as_array()
            .unwrap();
    assert!(correspondences.iter().all(|record| {
        record["sourceId"] != "matrix/left-synthetic-keys/r0c0"
            && record["mirrorId"] != "matrix/right-synthetic-keys/r0c0"
    }));
    assert!(correspondences.iter().any(|record| {
        record["sourceId"] == "matrix/left-synthetic-keys/r0c1"
            && record["mirrorId"] == "matrix/right-synthetic-keys/r0c1"
    }));

    let moved = request(
        &mut engine,
        json!({"id":"runtime-source-edit-after-membership","kind":"edit","command":{
            "baseRevision":removed["document"]["revision"],
            "transactionId":"runtime-source-edit-after-membership",
            "phase":"commit",
            "targetIds":["matrix/left-synthetic-keys/r0c1"],
            "operation":{
                "kind":"move-parts",
                "positions":[{"id":"matrix/left-synthetic-keys/r0c1","at":{"x":43.0,"y":7.0}}]
            }
        }}),
    );
    assert_eq!(moved["kind"], "scene", "{moved}");
    assert_eq!(
        part(&moved["document"], "matrix/left-synthetic-keys/r0c1")["pose"]["at"],
        json!({"x":43.0,"y":7.0})
    );
    assert_eq!(
        part(&moved["document"], "matrix/right-synthetic-keys/r0c1")["pose"]["at"],
        json!({"x":157.0,"y":7.0})
    );
}

#[test]
fn production_paired_previews_project_all_matrix_edit_families_without_publishing() {
    let mut document = runtime_supported_document();
    for cell in document["matrices"][0]["cells"].as_array_mut().unwrap() {
        cell["assemblies"] = json!([]);
    }
    let (mut engine, seed) = seed(&document, "left");
    let created = create(
        &mut engine,
        &seed,
        "runtime-preview-families-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let accepted = created["document"].clone();
    let mut matrix = accepted["matrices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|matrix| matrix["id"] == "left-synthetic-keys")
        .unwrap()
        .clone();
    matrix["columns"] = json!(3);
    let additions: Vec<Value> = (0..2)
        .map(|row| {
            let mut cell = matrix["cells"]
                .as_array()
                .unwrap()
                .iter()
                .find(|cell| cell["row"] == row && cell["column"] == 1)
                .unwrap()
                .clone();
            cell["column"] = json!(2);
            cell
        })
        .collect();
    matrix["cells"].as_array_mut().unwrap().extend(additions);

    let previews = [
        (
            "move-parts",
            json!({"kind":"move-parts","positions":[{"id":"right/U1","at":{"x":216.0,"y":14.0}}]}),
            "left/U1",
            "right/U1",
        ),
        (
            "set-matrix",
            json!({"kind":"set-matrix","matrix":matrix}),
            "matrix/left-synthetic-keys/r0c2",
            "matrix/right-synthetic-keys/r0c2",
        ),
        (
            "set-matrix-splay",
            json!({"kind":"set-matrix-splay","matrixId":"left-synthetic-keys","column":1,
                "change":{"kind":"angle","angle":12.0,"affect":"column"}}),
            "matrix/left-synthetic-keys/r0c1",
            "matrix/right-synthetic-keys/r0c1",
        ),
        (
            "set-matrix-placement",
            json!({"kind":"set-matrix-placement","matrixId":"left-synthetic-keys",
                "change":{"kind":"column-stagger","column":1,"value":4.0}}),
            "matrix/left-synthetic-keys/r0c1",
            "matrix/right-synthetic-keys/r0c1",
        ),
    ];

    for (index, (family, operation, source_id, mirror_id)) in previews.into_iter().enumerate() {
        let preview = request(
            &mut engine,
            json!({"id":format!("runtime-preview-{family}"),"kind":"edit","command":{
                "baseRevision":accepted["revision"],
                "transactionId":format!("runtime-preview-{family}"),
                "phase":"preview",
                "targetIds":[source_id, mirror_id],
                "operation":operation
            }}),
        );
        assert_eq!(preview["kind"], "preview", "{family}: {preview}");
        let transform = |id: &str| {
            preview["scene"]["transforms"]
                .as_array()
                .unwrap()
                .iter()
                .find(|transform| transform["id"] == id)
                .unwrap_or_else(|| {
                    panic!("{family}: missing preview transform for {id}: {preview}")
                })
        };
        let source_pose = &transform(source_id)["pose"];
        let mirror_pose = &transform(mirror_id)["pose"];
        assert!(source_pose.is_object(), "{family}: source draft pose");
        assert!(mirror_pose.is_object(), "{family}: mirror draft pose");
        if let Some(before) = find_part(&accepted, source_id) {
            assert_ne!(source_pose, &before["pose"], "{family}: source draft");
        } else {
            assert_eq!(family, "set-matrix", "only expansion adds preview parts");
        }
        if let Some(before) = find_part(&accepted, mirror_id) {
            assert_ne!(mirror_pose, &before["pose"], "{family}: mirror draft");
        } else {
            assert_eq!(family, "set-matrix", "only expansion adds preview parts");
        }
        assert_eq!(
            snapshot(&mut engine),
            accepted,
            "{family}: preview {index} must leave accepted state unchanged"
        );
    }
}

fn assert_authored_companion_preview(
    family: &str,
    operation: Value,
    source_id: &str,
    mirror_id: &str,
    expected_source: (f64, f64),
    expected_mirror: (f64, f64),
) {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        &format!("runtime-authored-preview-{family}"),
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let accepted = created["document"].clone();
    let transaction_id = format!("runtime-authored-preview-{family}");
    let preview: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":transaction_id,"kind":"edit","command":{
                "baseRevision":accepted["revision"],
                "transactionId":transaction_id,
                "phase":"preview",
                "targetIds":[source_id,mirror_id],
                "operation":operation
            }})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(
        snapshot(&mut engine),
        accepted,
        "{family}: authored-companion preview must leave accepted document and history unchanged"
    );
    assert_eq!(preview["kind"], "preview", "{family}: {preview}");
    let preview_pose = |id: &str| {
        let transform = preview["scene"]["transforms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|transform| transform["id"] == id)
            .unwrap_or_else(|| panic!("{family}: missing preview transform for {id}: {preview}"));
        (
            transform["pose"]["at"]["x"].as_f64().unwrap(),
            transform["pose"]["at"]["y"].as_f64().unwrap(),
        )
    };
    assert_point(
        preview_pose(source_id),
        expected_source,
        "authored source preview pose",
    );
    assert_point(
        preview_pose(mirror_id),
        expected_mirror,
        "authored mirror preview pose",
    );
}

#[test]
fn production_authored_companion_move_parts_preview_projects_both_halves() {
    assert_authored_companion_preview(
        "move-parts",
        json!({"kind":"move-parts","positions":[{"id":"right/U1","at":{"x":222.0,"y":4.0}}]}),
        "left/U1",
        "right/U1",
        (-22.0, 4.0),
        (222.0, 4.0),
    );
}

#[test]
fn production_authored_companion_set_matrix_preview_projects_expansion() {
    let mut matrix = runtime_supported_document()["matrices"][0].clone();
    matrix["columns"] = json!(3);
    let additions: Vec<Value> = (0..2)
        .map(|row| {
            let mut cell = matrix["cells"]
                .as_array()
                .unwrap()
                .iter()
                .find(|cell| cell["row"] == row && cell["column"] == 1)
                .unwrap()
                .clone();
            cell["column"] = json!(2);
            cell
        })
        .collect();
    matrix["cells"].as_array_mut().unwrap().extend(additions);
    matrix["partIds"].as_array_mut().unwrap().extend([
        json!("matrix/left-synthetic-keys/r0c2"),
        json!("matrix/left-synthetic-keys/r1c2"),
    ]);
    assert_authored_companion_preview(
        "set-matrix",
        json!({"kind":"set-matrix","matrix":matrix}),
        "matrix/left-synthetic-keys/r0c2",
        "matrix/right-synthetic-keys/r0c2",
        (38.0, 0.0),
        (162.0, 0.0),
    );
}

#[test]
fn production_authored_companion_set_matrix_splay_preview_projects_both_halves() {
    let x = 19.0 + 21.0 * 12.0_f64.to_radians().sin();
    let y = 21.0 * 12.0_f64.to_radians().cos();
    assert_authored_companion_preview(
        "set-matrix-splay",
        json!({"kind":"set-matrix-splay","matrixId":"left-synthetic-keys","column":1,
            "change":{"kind":"angle","angle":12.0,"affect":"column"}}),
        "matrix/left-synthetic-keys/r1c1",
        "matrix/right-synthetic-keys/r1c1",
        (x, y),
        (200.0 - x, y),
    );
}

#[test]
fn production_authored_companion_set_matrix_placement_preview_projects_both_halves() {
    assert_authored_companion_preview(
        "set-matrix-placement",
        json!({"kind":"set-matrix-placement","matrixId":"left-synthetic-keys",
            "change":{"kind":"column-stagger","column":1,"value":4.0}}),
        "matrix/left-synthetic-keys/r0c1",
        "matrix/right-synthetic-keys/r0c1",
        (19.0, 4.0),
        (181.0, 4.0),
    );
}

#[test]
fn production_paired_noop_commit_preserves_revision_history_and_redo() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        "runtime-noop-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let accepted = created["document"].clone();
    let committed = request(
        &mut engine,
        json!({"id":"runtime-noop-history-setup","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"runtime-noop-history-setup",
            "phase":"commit",
            "targetIds":["left/U1"],
            "operation":{"kind":"move-parts","positions":[{"id":"left/U1","at":{"x":-22.0,"y":4.0}}]}
        }}),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    let undone = request(
        &mut engine,
        json!({"id":"runtime-noop-history-undo","kind":"undo"}),
    );
    assert_eq!(
        part(&undone["document"], "left/U1")["pose"]["at"],
        json!({"x":-24.0,"y":0.0})
    );
    assert_eq!(
        part(&undone["document"], "right/U1")["pose"]["at"],
        json!({"x":224.0,"y":0.0})
    );
    let before_noop = snapshot(&mut engine);

    let noop = request(
        &mut engine,
        json!({"id":"runtime-noop-commit","kind":"edit","command":{
            "baseRevision":before_noop["revision"],
            "transactionId":"runtime-noop-commit",
            "phase":"commit",
            "targetIds":["left/U1"],
            "operation":{"kind":"apply-mechanical-edit","edit":{
                "target":{"kind":"instance","instanceId":"left"},
                "change":{"set-dimension":{"field":"wall-thickness","value":2.0}}
            }}
        }}),
    );
    assert_eq!(noop["kind"], "scene", "{noop}");
    assert_eq!(noop["document"]["revision"], before_noop["revision"]);
    assert_eq!(
        snapshot(&mut engine),
        before_noop,
        "no-op must leave document and history position unchanged"
    );

    let redone = request(
        &mut engine,
        json!({"id":"runtime-noop-history-redo","kind":"redo"}),
    );
    assert_eq!(redone["kind"], "scene", "{redone}");
    assert_eq!(
        part(&redone["document"], "left/U1")["pose"]["at"],
        json!({"x":-22.0,"y":4.0})
    );
    assert_eq!(
        part(&redone["document"], "right/U1")["pose"]["at"],
        json!({"x":222.0,"y":4.0})
    );
}
