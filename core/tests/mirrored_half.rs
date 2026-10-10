//! `CreateMirroredHalf` (ticket 18) through Core's request surface.
//!
//! The source is the shipped measured half, trimmed to one board, so the second half is
//! checked against the right (or left) half the recipes ship today. Key positions are compared
//! by physical correspondence, because the mirror keeps the source's matrix numbering.
use crate::included_keyboards::projects;

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

/// The shipped measured project trimmed to one board: its parts, matrices, layouts, outline,
/// nets, boards and physical instances, with the pair removed.
fn half_only(document: &Value, board: &str) -> Value {
    let mut doc = document.clone();
    let kept_board = doc["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == board)
        .unwrap()
        .clone();
    let ids = |key: &str| -> BTreeSet<String> {
        kept_board[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect()
    };
    let parts = ids("partIds");
    let outlines = ids("outlineIds");
    let nets = ids("netIds");
    let keep = |list: &str, pick: &dyn Fn(&Value) -> bool| -> Value {
        Value::Array(
            document[list]
                .as_array()
                .map(|items| items.iter().filter(|item| pick(item)).cloned().collect())
                .unwrap_or_default(),
        )
    };
    doc["boards"] = json!([kept_board]);
    doc["parts"] = keep("parts", &|p| parts.contains(p["id"].as_str().unwrap()));
    doc["matrices"] = keep("matrices", &|m| m["boardId"] == board);
    doc["layouts"] = keep("layouts", &|l| l["boardId"] == board);
    doc["outline"] = keep("outline", &|o| outlines.contains(o["id"].as_str().unwrap()));
    doc["boardOutlines"] = keep("boardOutlines", &|o| o["boardId"] == board);
    doc["nets"] = keep("nets", &|n| nets.contains(n["id"].as_str().unwrap()));
    for net in doc["nets"].as_array_mut().unwrap() {
        let pins: Vec<Value> = net["pins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|pin| parts.contains(pin["partId"].as_str().unwrap()))
            .cloned()
            .collect();
        net["pins"] = json!(pins);
    }
    let hardware = &mut doc["hardware"];
    let instances: Vec<Value> = hardware["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["boardId"] == board)
        .cloned()
        .map(|mut i| {
            i["constructionOwnership"] = json!("local");
            i
        })
        .collect();
    hardware["instances"] = json!(instances);
    hardware["boards"] = json!(
        hardware["boards"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|b| b["boardId"] == board)
            .cloned()
            .collect::<Vec<_>>()
    );
    hardware["pairs"] = json!([]);
    doc
}

/// The switch positions of one half: `matrix/<half>-<cluster>/rXcY`, as (x, y, rotation).
fn switches(document: &Value, half: &str) -> Vec<(f64, f64, f64)> {
    document["parts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| {
            let id = p["id"].as_str().unwrap();
            let segments: Vec<&str> = id.split('/').collect();
            segments.len() == 3
                && segments[0] == "matrix"
                && segments[1].starts_with(&format!("{half}-"))
        })
        .map(|p| {
            (
                p["pose"]["at"]["x"].as_f64().unwrap(),
                p["pose"]["at"]["y"].as_f64().unwrap(),
                p["pose"]["rotation"].as_f64().unwrap(),
            )
        })
        .collect()
}

fn part<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
        .unwrap()
}

fn angles_match(a: f64, b: f64) -> bool {
    let turn = (a - b).rem_euclid(360.0);
    turn.min(360.0 - turn) < 1e-9
}

/// Every expected switch has exactly one actual switch at the same pose, and the counts agree.
fn assert_same_switches(actual: &[(f64, f64, f64)], expected: &[(f64, f64, f64)], label: &str) {
    assert_eq!(actual.len(), expected.len(), "{label}: switch count");
    let mut used = vec![false; actual.len()];
    for (x, y, rotation) in expected {
        let found = actual.iter().enumerate().position(|(index, (ax, ay, ar))| {
            !used[index]
                && (ax - x).abs() < 1e-9
                && (ay - y).abs() < 1e-9
                && angles_match(*ar, *rotation)
        });
        let index =
            found.unwrap_or_else(|| panic!("{label}: no switch at ({x}, {y}) rotated {rotation}"));
        used[index] = true;
    }
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

/// The shipped pair's map, which takes its first member to its second. A vertical-axis
/// reflection is its own inverse, so it also takes a source on the right to the left.
fn shipped_mapping(shipped: &Value) -> Value {
    shipped["hardware"]["pairs"][0]["mapping"].clone()
}

/// The shipped project's half as a one-half project would hold it: one board with the parts,
/// matrices and outline it owns, and the mechanical configuration the shipped pair resolves
/// for it (the linked openings and closure mounts folded into the half's own configuration).
fn seed_document(shipped: &Value, board: &str) -> Value {
    let mut doc = half_only(shipped, board);
    let typed: ProjectDoc = serde_json::from_value(shipped.clone()).unwrap();
    let configuration = effective_inputs(
        &typed,
        &[],
        &MechanicalTarget::Instance {
            instance_id: board.into(),
        },
    )
    .unwrap()
    .configuration;
    for instance in doc["hardware"]["instances"].as_array_mut().unwrap() {
        if instance["id"] == board {
            instance["mechanical"] = serde_json::to_value(&configuration).unwrap();
        }
    }
    doc
}

fn seed(shipped: &Value, board: &str) -> (CoreEngine, Value) {
    open(&seed_document(shipped, board))
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

/// Numbers match to 1e-9 (relative); everything else exactly.
fn close(actual: &Value, expected: &Value, path: &str) {
    match expected {
        Value::Number(number) => {
            let expected = number.as_f64().unwrap();
            let actual = actual
                .as_f64()
                .unwrap_or_else(|| panic!("{path}: expected a number, got {actual}"));
            assert!(
                (actual - expected).abs() <= 1e-9 * expected.abs().max(1.0),
                "{path}: {actual} != {expected}"
            );
        }
        Value::Object(object) => {
            assert_eq!(
                actual.as_object().map(|o| o.len()),
                Some(object.len()),
                "{path}: keys {actual}"
            );
            for (key, value) in object {
                close(&actual[key], value, &format!("{path}/{key}"));
            }
        }
        Value::Array(items) => {
            let actual = actual
                .as_array()
                .unwrap_or_else(|| panic!("{path}: expected an array"));
            assert_eq!(actual.len(), items.len(), "{path}");
            for (index, (actual, expected)) in actual.iter().zip(items).enumerate() {
                close(actual, expected, &format!("{path}/{index}"));
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}

/// A part's pose, reference, side and footprint must agree with the shipped one.
fn assert_part_matches(actual: &Value, expected: &Value, id: &str) {
    assert_eq!(actual["reference"], expected["reference"], "{id} reference");
    assert_eq!(actual["side"], expected["side"], "{id} side");
    assert_eq!(
        actual["definitionId"], expected["definitionId"],
        "{id} footprint"
    );
    for axis in ["x", "y"] {
        let (a, b) = (
            actual["pose"]["at"][axis].as_f64().unwrap(),
            expected["pose"]["at"][axis].as_f64().unwrap(),
        );
        assert!((a - b).abs() < 1e-9, "{id} {axis}: {a} vs {b}");
    }
    assert!(
        angles_match(
            actual["pose"]["rotation"].as_f64().unwrap(),
            expected["pose"]["rotation"].as_f64().unwrap(),
        ),
        "{id} rotation"
    );
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
fn corne_second_half_reproduces_the_shipped_right_half() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = seed(shipped, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        shipped_mapping(shipped),
    )
    .unwrap();
    let mirrored = &reply["document"];
    // Every key and diode lands where the shipped right half has one, by correspondence (the
    // mirror keeps the source's numbering; the recipe reversed it).
    assert_same_switches(
        &switches(mirrored, "right"),
        &switches(shipped, "right"),
        "keys",
    );
    // The recipe places a diode at the same offset and turn beside every key of both halves
    // (it is not a reflection), so the shipped right diodes are not compared: the mirror's are
    // checked against the pair rule by `assert_reflects_every_part`.
    assert_reflects_every_part(mirrored, "pair-right", "corne");
    // The controller, reset switch and TRRS jack match the recipe's reflected bay by id.
    for id in ["right/U1", "right/RST", "right/TRRS"] {
        assert_part_matches(part(mirrored, id), part(shipped, id), id);
    }
    // The source half is untouched by its own reflection.
    assert_same_switches(
        &switches(mirrored, "left"),
        &switches(&seed, "left"),
        "source half",
    );
    // The pair carries the same linked closure mounts and openings, authored by the source.
    close(
        &pair(mirrored, "pair-right")["features"],
        &shipped["hardware"]["pairs"][0]["features"],
        "features",
    );
}

/// Every corresponded part sits at the pair map's image of its source: the point through
/// `q = R(rotation) * (-p.x, p.y) + at`, the heading by `rotation - angle`, on the same side.
fn assert_reflects_every_part(document: &Value, pair_id: &str, label: &str) {
    let record = pair(document, pair_id);
    let map = &record["mapping"];
    let (at_x, at_y, turn) = (
        map["at"]["x"].as_f64().unwrap(),
        map["at"]["y"].as_f64().unwrap(),
        map["rotation"].as_f64().unwrap(),
    );
    let (sin, cos) = turn.to_radians().sin_cos();
    let mut compared = 0;
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
        assert!(
            angles_match(
                mirror["pose"]["rotation"].as_f64().unwrap(),
                turn - source["pose"]["rotation"].as_f64().unwrap(),
            ),
            "{label} {id}: rotation {} from {}",
            mirror["pose"]["rotation"],
            source["pose"]["rotation"]
        );
        assert_eq!(mirror["side"], source["side"], "{label} {id}: side");
        compared += 1;
    }
    assert!(compared > 20, "{label}: only {compared} parts compared");
}

#[test]
fn a_rotated_pair_map_reflects_keys_companions_and_other_parts_by_the_pair_rule() {
    for (name, source_board) in [("corne", "left"), ("lily58", "right"), ("klor", "right")] {
        let shipped = &projects()[name];
        let (mut engine, seed) = seed(shipped, source_board);
        let reply = create(
            &mut engine,
            &seed,
            "pair-rotated",
            source_board,
            new_board("mirror", "mirror"),
            json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0}),
        )
        .unwrap();
        assert_reflects_every_part(&reply["document"], "pair-rotated", name);
    }
}

#[test]
fn a_vertical_axis_map_reflects_every_part_by_the_pair_rule() {
    for (name, source_board) in [("corne", "left"), ("klor", "right")] {
        let shipped = &projects()[name];
        let (mut engine, seed) = seed(shipped, source_board);
        let reply = create(
            &mut engine,
            &seed,
            "pair-flat",
            source_board,
            new_board("mirror", "mirror"),
            mapping(300.0),
        )
        .unwrap();
        assert_reflects_every_part(&reply["document"], "pair-flat", name);
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
fn source_right_splits_reproduce_the_shipped_left_half() {
    for name in ["lily58", "klor"] {
        let shipped = &projects()[name];
        let (mut engine, seed) = seed(shipped, "right");
        let reply = create(
            &mut engine,
            &seed,
            "pair-left",
            "right",
            new_board("left", "left"),
            shipped_mapping(shipped),
        )
        .unwrap();
        let mirrored = &reply["document"];
        assert_same_switches(
            &switches(mirrored, "left"),
            &switches(shipped, "left"),
            name,
        );
        for id in ["left/U1", "left/RST", "left/TRRS"] {
            assert_part_matches(
                part(mirrored, id),
                part(shipped, id),
                &format!("{name} {id}"),
            );
        }
        assert_reflects_every_part(mirrored, "pair-left", name);
        assert_eq!(
            pair(mirrored, "pair-left")["constructionSource"],
            "right",
            "{name}"
        );
        assert_eq!(instance(mirrored, "left")["half"], "left", "{name}");
        assert_eq!(instance(mirrored, "left")["role"], "central", "{name}");
        // The mirror's closure mounts are the reflection of the source's, which are the
        // shipped left half's again.
        let shipped_left = effective(shipped, "left");
        let created_left = effective(mirrored, "left");
        let (expected, actual) = (
            closure_positions(&shipped_left),
            closure_positions(&created_left),
        );
        assert_eq!(expected.len(), actual.len(), "{name} closure mounts");
        for (e, a) in expected.iter().zip(&actual) {
            assert!(
                (e.0 - a.0).abs() < 1e-6 && (e.1 - a.1).abs() < 1e-6,
                "{name}: {e:?} {a:?}"
            );
        }
        // The shipped pair's generated holes exist on both created boards.
        assert_eq!(
            closure_holes(mirrored, "left"),
            closure_holes(shipped, "left"),
            "{name}"
        );
        assert_eq!(
            closure_holes(mirrored, "right"),
            closure_holes(shipped, "right"),
            "{name}"
        );
    }
}

#[test]
fn the_pair_holds_the_shipped_linked_features_and_generated_holes_are_not_copied() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = seed(shipped, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        shipped_mapping(shipped),
    )
    .unwrap();
    let document = &reply["document"];
    // Linked, authored by the source, with the shipped identities.
    let record = pair(document, "pair-right");
    let shipped_features = &shipped["hardware"]["pairs"][0]["features"];
    let key = |features: &Value| -> Vec<Value> {
        let mut list: Vec<Value> = features.as_array().unwrap().clone();
        list.sort_by_key(|f| f["id"].as_str().unwrap().to_owned());
        list
    };
    close(
        &Value::Array(key(&record["features"])),
        &Value::Array(key(shipped_features)),
        "features",
    );
    // Both instances have their openings and closure mounts in the pair, not in their lists.
    for half in ["left", "right"] {
        let stored = &instance(document, half)["mechanical"];
        assert_eq!(stored["closureMounts"], json!([]), "{half}");
        assert_eq!(stored["openings"], json!([]), "{half}");
        assert_eq!(stored["boardId"], half, "{half}");
        assert_eq!(instance(document, half)["constructionOwnership"], "pair");
    }
    // The mirror reads the reflection of the shipped right half's configuration.
    close(
        &json!(closure_positions(&effective(document, "right"))),
        &json!(closure_positions(&effective(shipped, "right"))),
        "right closure mounts",
    );
    // Holes are regenerated from the pair: the right board has exactly the shipped holes, none at
    // the source's coordinates, and the left board kept its own.
    assert_eq!(
        closure_holes(document, "right"),
        closure_holes(shipped, "right")
    );
    assert_eq!(
        closure_holes(document, "left"),
        closure_holes(shipped, "left")
    );
    assert_eq!(closure_holes(document, "right").len(), 4);
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
}

#[test]
fn changed_ids_are_exactly_the_new_items_the_source_ids_and_the_generated_holes() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = seed(shipped, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        shipped_mapping(shipped),
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
    assert!(
        expected
            .iter()
            .any(|id| id.starts_with("case-closure/right/"))
    );
    let changed: BTreeSet<String> = reply["scene"]["changedIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(changed, expected);
    assert!(
        changed.contains("matrix/right-keys/r0c0") && changed.contains("matrix/left-keys/r0c0")
    );
    assert!(changed.contains("right/TRRS") && changed.contains("left/TRRS"));
}

#[test]
fn one_undo_step_removes_the_half_and_redo_restores_it() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = seed(shipped, "left");
    let before = snapshot(&mut engine);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        shipped_mapping(shipped),
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
    let shipped = &projects()["corne"];
    let (mut engine, seed) = seed(shipped, "left");
    let good = shipped_mapping(shipped);
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
    // Closure mounts need the mounting-hole definition, as they do for `LinkMirrorPair`.
    let mut without = op("p", "left", new_board("right", "right"));
    without.as_object_mut().unwrap().remove("mountingHole");
    assert!(refused(&mut engine, &seed, without).contains("mounting-hole"));
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
fn a_preview_changes_nothing_and_a_commit_still_follows() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = seed(shipped, "left");
    let before = serde_json::to_string(&snapshot(&mut engine)).unwrap();
    let op = operation(
        "pair-right",
        "left",
        new_board("right", "right"),
        shipped_mapping(shipped),
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
fn reuse_board_creates_a_flipped_instance_with_linked_features_and_copies_nothing() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = seed(shipped, "left");
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
    // The shipped linked features, not cleared copies: the flipped half reads the reflection.
    let record = pair(document, "pair-flipped");
    assert_eq!(record["members"], json!(["left", "flipped"]));
    assert_eq!(record["constructionSource"], "left");
    assert_eq!(record["halves"]["sourceBoardId"], "left");
    assert_eq!(record["halves"]["mirrorBoardId"], "left");
    assert_eq!(record["features"].as_array().unwrap().len(), 6);
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
    assert_eq!(source["openings"].as_array().unwrap().len(), 2);
    assert_eq!(mirror["openings"].as_array().unwrap().len(), 2);
    assert_eq!(closure_positions(&mirror).len(), 4);
    // The flipped half's holes land on the source's: a flipped board reflects the map back.
    assert_eq!(closure_holes(document, "left").len(), 4);
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
    let shipped = &projects()["corne"];
    let mut doc = seed_document(shipped, "left");
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
        shipped_mapping(shipped),
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
    let shipped = &projects()["corne"];
    let mut doc = seed_document(shipped, "left");
    doc["keycaps"] = json!({
        "boards": {"left": {"color": "#111111", "legendColor": "#222222", "clearance": 0.6}},
        "matrices": {"left-keys": {"profile": "dsa", "mount": "mx", "firstRow": 2, "wallThickness": 1.4}},
        "keys": {"matrix/left-keys/r0c0": {"legend": "Q", "color": "#ff0000", "row": 3}},
    });
    let board = &mut doc["hardware"]["boards"][0];
    board["locks"] = json!({"row/0": "P10", "peripheral/left/TRRS/split-rx": "P14"});
    board["keyBindings"] = json!({"matrix/left-keys/r0c0": "&kp Q"});
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
        shipped_mapping(shipped),
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
    let expected = json!({"boardId": "right", "id": "right-keys-layout", "matrixId": "right-keys", "name": "keys", "partIds": ["right/U1", "right/RST", "right/TRRS"]});
    assert_eq!(layout("right-keys-layout"), expected);
    assert_eq!(layout("right-thumbs-layout")["partIds"], json!([]));
    assert_eq!(layout("left-keys-layout"), seed["layouts"][0]);
    assert_eq!(document["layouts"].as_array().unwrap().len(), 4);
    // Keycaps: settings follow the mapped ids; a legend is copied as it is (a binding is not).
    let keycaps = &document["keycaps"];
    assert_eq!(keycaps["boards"]["right"], keycaps["boards"]["left"]);
    assert_eq!(
        keycaps["matrices"]["right-keys"],
        keycaps["matrices"]["left-keys"]
    );
    assert_eq!(
        keycaps["keys"]["matrix/right-keys/r0c0"]["color"],
        "#ff0000"
    );
    assert_eq!(keycaps["keys"]["matrix/right-keys/r0c0"]["row"], 3);
    assert_eq!(keycaps["keys"]["matrix/right-keys/r0c0"]["legend"], "Q");
    assert_eq!(keycaps["keys"]["matrix/left-keys/r0c0"]["legend"], "Q");
    // Electrical: the same pins on the renamed functions, nothing of the source in the names.
    let boards = document["hardware"]["boards"].as_array().unwrap();
    let (left, right) = (&boards[0], &boards[1]);
    assert_eq!(right["boardId"], "right");
    assert_eq!(right["controllerPartId"], "right/U1");
    assert_eq!(right["keyBindings"], json!({}));
    assert_eq!(right["protectedHandoff"], Value::Null);
    assert_eq!(
        left["keyBindings"],
        json!({"matrix/left-keys/r0c0": "&kp Q"})
    );
    assert_eq!(
        right["locks"],
        json!({"row/0": "P10", "peripheral/right/TRRS/split-rx": "P14"})
    );
    assert_eq!(
        right["assignments"]["column/0"],
        left["assignments"]["column/0"]
    );
    assert_eq!(
        right["assignments"]["peripheral/right/TRRS/split-tx"],
        left["assignments"]["peripheral/left/TRRS/split-tx"]
    );
    assert!(
        right["assignments"]
            .as_object()
            .unwrap()
            .keys()
            .all(|k| !k.contains("left"))
    );
    let right_nets: Vec<&Value> = document["nets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| {
            n["id"]
                .as_str()
                .unwrap()
                .starts_with("generated/electrical/right/")
        })
        .collect();
    assert_eq!(right_nets.len(), 35);
    for net in right_nets {
        assert!(!net["name"].as_str().unwrap().contains("LEFT"), "{net}");
        for pin in net["pins"].as_array().unwrap() {
            assert!(!pin["partId"].as_str().unwrap().contains("left"), "{net}");
        }
    }
    assert!(
        document["nets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["name"] == "PERIPHERAL_RIGHT_TRRS_SPLIT-RX")
    );
}

#[test]
fn a_board_with_nothing_on_it_and_a_matrix_id_without_the_board_token_still_mirror() {
    // An empty board mirrors to an empty board.
    let shipped = &projects()["corne"];
    let mut doc = seed_document(shipped, "left");
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
        shipped_mapping(shipped),
    )
    .unwrap();
    assert_eq!(reply["document"]["boards"].as_array().unwrap().len(), 2);
    // A matrix whose id does not contain the board id follows its members.
    let mut doc = seed_document(shipped, "left");
    let text = serde_json::to_string(&doc)
        .unwrap()
        .replace("left-keys", "alpha")
        .replace("left-thumbs", "thumbs");
    doc = serde_json::from_str(&text).unwrap();
    let (mut engine, seed) = open(&doc);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        shipped_mapping(shipped),
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
    let shipped = &projects()["corne"];
    let (mut engine, seed) = seed(shipped, "left");
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
    let shipped = &projects()["corne"];
    let (mut engine, mut document) = seed(shipped, "left");
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
            .unwrap()
            .clone()
    };

    // Live outline, in the part frame: the attached point flips handedness, the free one maps.
    let live = find(&out["outline"], "right-outline");
    let connection = &live["connections"][0];
    assert_eq!(connection["points"][0]["partId"], "right/U1");
    assert_eq!(connection["points"][0]["at"], json!({"x": -4.0, "y": 2.0}));
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
    // Reflection reverses the winding, so the points come back in the reverse order.
    assert_eq!(polygon["points"][0], json!({"x": -5.0, "y": 7.0}));
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
        find(&out["outline"], "left-outline"),
        find(&seed_doc["outline"], "left-outline")
    );
}

#[test]
fn a_mirrored_outline_identity_that_is_already_used_refuses_the_whole_record() {
    let shipped = &projects()["corne"];
    let (mut engine, mut document) = seed(shipped, "left");
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
            shipped_mapping(shipped),
        ),
    );
    assert!(error.contains("right-link"), "{error}");
}

#[test]
fn hosted_modules_and_embedded_circuits_are_refused_by_name() {
    let shipped = &projects()["corne"];
    let mut doc = seed_document(shipped, "left");
    doc["embeddedCircuits"] = json!([{"id": "circuit-1", "definitionId": "d", "hostBoardId": "left",
        "partIds": [], "netIds": [], "ports": {}}]);
    let opened: Value = serde_json::from_str(
        &CoreEngine::new().request(&json!({"id":"o","kind":"open","document":doc}).to_string()),
    )
    .unwrap();
    assert_ne!(opened["kind"], "error", "{opened}");
    let (mut engine, seed) = open(&doc);
    let error = refused(
        &mut engine,
        &seed,
        operation(
            "p",
            "left",
            new_board("right", "right"),
            shipped_mapping(shipped),
        ),
    );
    assert!(
        error.contains("embedded circuit circuit-1") && error.contains("cannot be mirrored yet"),
        "{error}"
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
/// CURRENT BEHAVIOUR, NOT A DECISION: the diode is at local (+7.4, -1.5) on the source half, so the
/// mirror puts it at local (-7.4, -1.5), the other side of its switch. The shipped right halves keep
/// the diode at (+7.4, -1.5) on the same side as the left half. Which one is intended is an open user
/// decision (ticket 29, Outcome); the assertion below pins what Core does today, not the recipes.
#[test]
fn key_switches_mirror_their_tilt_without_a_half_turn_and_keep_their_companion_side() {
    let cases: [(&str, Option<f64>); 4] = [
        ("corne", None),
        ("corne", Some(27.0)),
        ("sofle-rgb", None),
        ("sofle-rgb", Some(27.0)),
    ];
    for (name, turn) in cases {
        let shipped = &projects()[name];
        let (mut engine, seed) = seed(shipped, "left");
        // Two switches carry a 15 and a -20 degree cell turn (the other two stay flat), so the
        // tilts are not the zero the shipped matrices use.
        let tilted = with_cell_turns(
            &mut engine,
            &seed,
            "left-keys",
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
        let source = |r: u64, c: u64| part(&tilted, &format!("matrix/left-keys/r{r}c{c}"));
        let mirror = |r: u64, c: u64| part(mirrored, &format!("matrix/right-keys/r{r}c{c}"));
        let tilt = |r: u64, c: u64| source(r, c)["pose"]["rotation"].as_f64().unwrap();
        assert!(
            angles_match(tilt(0, 0) - tilt(1, 0), 15.0)
                && angles_match(tilt(0, 1) - tilt(1, 1), -20.0),
            "{label}: the source switches must carry the 15 and -20 degree turns"
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
            // The companions: the diode is on every switch, the LED only on RGB variants.
            for (suffix, local) in [("diode", (7.4, -1.5)), ("led", (0.0, -4.75))] {
                let source_id = format!("matrix/left-keys/r{r}c{c}/{suffix}");
                let Some(companion) = find_part(&tilted, &source_id) else {
                    continue;
                };
                let mirror_id = format!("matrix/right-keys/r{r}c{c}/{suffix}");
                let mirror_companion = find_part(mirrored, &mirror_id)
                    .unwrap_or_else(|| panic!("{label}: no mirror part {mirror_id}"));
                let (lx, ly) = in_switch_frame(companion, source(r, c));
                assert!(
                    (lx - local.0).abs() < 1e-6 && (ly - local.1).abs() < 1e-6,
                    "{label} {source_id}: the shipped offset changed to ({lx}, {ly})"
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

/// Every shipped split's mirrored half reproduces the shipped other half: key switches by
/// correspondence, and every corresponded part (companions included) at the pair map's image.
#[test]
fn every_shipped_split_mirror_follows_the_switch_and_companion_rules() {
    for name in [
        "corne",
        "lily58",
        "klor",
        "sweep",
        "chocofi",
        "totem",
        "cantor",
        "sofle-v2",
        "sofle-rgb",
        "sofle-choc",
    ] {
        let source = if matches!(name, "lily58" | "klor") {
            "right"
        } else {
            "left"
        };
        let other = if source == "left" { "right" } else { "left" };
        let shipped = &projects()[name];
        let (mut engine, seed) = seed(shipped, source);
        let reply = create(
            &mut engine,
            &seed,
            "pair-shipped",
            source,
            new_board(other, other),
            shipped_mapping(shipped),
        )
        .unwrap();
        let mirrored = &reply["document"];
        assert_reflects_every_part(mirrored, "pair-shipped", name);
        assert_same_switches(&switches(mirrored, other), &switches(shipped, other), name);
    }
}

/// A mirrored footprint that is handed and sits off the 90-degree grid is a finding for a person to
/// confirm. A switch or a diode never is: their outline is the same under a half turn and a mirror.
#[test]
fn a_handed_mirrored_footprint_off_the_right_angle_grid_is_a_finding() {
    let shipped = &projects()["sofle-rgb"];
    let (mut engine, seed) = seed(shipped, "left");
    let tilted = with_cell_turns(&mut engine, &seed, "left-keys", &[(0, 0, 15.0)]);
    let reply = create(
        &mut engine,
        &tilted,
        "pair-finding",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap();
    let ids: Vec<&str> = reply["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|finding| finding["id"].as_str())
        .collect();
    assert!(
        ids.contains(&"mirror/matrix/right-keys/r0c0/led/heading-uncertain"),
        "the mirrored LED at -195 degrees is handed and off the grid: {ids:?}"
    );
    assert!(
        !ids.iter()
            .any(|id| id.contains("/r0c0/diode") || id.ends_with("matrix/right-keys/r0c0")),
        "a symmetric diode or switch is never a finding: {ids:?}"
    );
}

/// The finding is once per part, located on the part, a Pcb-scope Warning (so it never gates an
/// export), absent from a document with no mirror pair, and absent for the MX stabilisers, whose pads
/// are at exactly `+-x` so their mirror image is themselves.
#[test]
fn the_heading_finding_is_one_located_warning_per_part_and_absent_without_a_pair() {
    let shipped = &projects()["sofle-rgb"];
    let (mut engine, seed) = seed(shipped, "left");
    let tilted = with_cell_turns(&mut engine, &seed, "left-keys", &[(0, 0, 15.0)]);
    let heading = |scene: &Value| -> Vec<Value> {
        scene["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["id"].as_str().unwrap().ends_with("/heading-uncertain"))
            .cloned()
            .collect()
    };
    // No mirror pair: nothing is added, whatever the tilt.
    let single = request(
        &mut CoreEngine::new(),
        json!({"id":"o","kind":"open","document":tilted}),
    );
    assert!(
        heading(&single["scene"]).is_empty(),
        "{:?}",
        heading(&single["scene"])
    );
    for (name, project) in projects().iter() {
        let reply = request(
            &mut CoreEngine::new(),
            json!({"id":"o","kind":"open","document":project}),
        );
        assert!(
            heading(&reply["scene"]).is_empty(),
            "{name}: a shipped project (no created mirror pair) gained a heading finding"
        );
    }
    let reply = create(
        &mut engine,
        &tilted,
        "pair-finding",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap();
    let flagged = heading(&reply["scene"]);
    let ids: BTreeSet<&str> = flagged.iter().map(|f| f["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids.len(),
        flagged.len(),
        "one finding per part: {flagged:?}"
    );
    let finding = flagged
        .iter()
        .find(|f| f["id"] == "mirror/matrix/right-keys/r0c0/led/heading-uncertain")
        .unwrap_or_else(|| panic!("the tilted mirror LED is not flagged: {ids:?}"));
    assert_eq!(finding["severity"], "warning");
    assert_eq!(finding["scope"], "pcb");
    assert_eq!(
        finding["targetIds"],
        json!(["matrix/right-keys/r0c0/led"]),
        "the finding names its part"
    );
    // A stabiliser (imported, no generator) is mirror-symmetric: relabel the flagged LED as one.
    let mut document = reply["document"].clone();
    let led = find_part(&document, "matrix/right-keys/r0c0/led").unwrap();
    let definition_id = led["definitionId"].as_str().unwrap().to_owned();
    let mut stabiliser = document["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == definition_id.as_str())
        .unwrap()
        .clone();
    stabiliser["id"] = json!("kicad:marbastlib/STAB_MX_2u");
    stabiliser.as_object_mut().unwrap().remove("generator");
    document["definitions"]
        .as_array_mut()
        .unwrap()
        .push(stabiliser);
    for part in document["parts"].as_array_mut().unwrap() {
        if part["id"] == "matrix/right-keys/r0c0/led" {
            part["definitionId"] = json!("kicad:marbastlib/STAB_MX_2u");
        }
    }
    let after = request(
        &mut CoreEngine::new(),
        json!({"id":"o","kind":"open","document":document}),
    );
    // What remains is the unmodified Sofle's own tilted thumb LEDs (-165 and -240 degrees), which
    // the finding is meant to raise; the relabelled r0c0 part is no longer one of them.
    let remaining: BTreeSet<String> = heading(&after["scene"])
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        remaining,
        BTreeSet::from([
            "mirror/matrix/right-thumbs/r0c3/led/heading-uncertain".to_owned(),
            "mirror/matrix/right-thumbs/r0c4/led/heading-uncertain".to_owned(),
        ]),
        "a mirror-symmetric stabiliser is flagged, or the thumb LEDs are not"
    );
}
