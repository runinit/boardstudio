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

/// The reflection constant of the pair map: the shipped mirror's least x plus the source's
/// greatest x, so `x' = axis - x` maps the source set exactly onto the shipped set.
fn axis(source: &[(f64, f64, f64)], shipped: &[(f64, f64, f64)]) -> f64 {
    let least = shipped.iter().map(|k| k.0).fold(f64::INFINITY, f64::min);
    let greatest = source.iter().map(|k| k.0).fold(f64::NEG_INFINITY, f64::max);
    least + greatest
}

fn mapping(axis: f64) -> Value {
    json!({"at": {"x": axis, "y": 0.0}, "rotation": 0.0})
}

fn new_board(board: &str, instance: &str) -> Value {
    json!({"kind":"new-board","boardId":board,"instanceId":instance})
}

/// Submits one `CreateMirroredHalf` commit and returns the reply, or the refusal message.
fn create(
    engine: &mut CoreEngine,
    document: &Value,
    pair: &str,
    source: &str,
    mode: Value,
    mapping: Value,
) -> Result<Value, String> {
    let operation = json!({
        "kind": "create-mirrored-half",
        "pairId": pair,
        "sourceInstanceId": source,
        "mode": mode,
        "mapping": mapping,
        "idRule": "board-prefix",
    });
    let reply: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"half","kind":"edit","command":{
            "baseRevision":document["revision"],"transactionId":"half","phase":"commit",
            "targetIds":[],"operation":operation}})
            .to_string(),
        ),
    )
    .unwrap();
    if reply["kind"] == "error" {
        return Err(reply["message"].as_str().unwrap().to_owned());
    }
    assert_eq!(reply["kind"], "scene", "{reply}");
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

#[test]
fn corne_second_half_reproduces_the_shipped_right_keys() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = open(&half_only(shipped, "left"));
    let shipped_right = switches(shipped, "right");
    let left = switches(&seed, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        mapping(axis(&left, &shipped_right)),
    )
    .unwrap();
    let mirrored = &reply["document"];
    assert_same_switches(
        &switches(mirrored, "right"),
        &shipped_right,
        "corne right half",
    );
    // The controller and reset switch are not matrix members: their poses and references match
    // the shipped right half by id, as the recipe places them.
    for id in ["right/U1", "right/RST"] {
        let shipped_part = part(shipped, id);
        let mirrored_part = part(mirrored, id);
        assert_eq!(
            mirrored_part["reference"], shipped_part["reference"],
            "{id}"
        );
        assert_eq!(mirrored_part["side"], shipped_part["side"], "{id}");
        assert_eq!(
            mirrored_part["definitionId"], shipped_part["definitionId"],
            "{id}"
        );
        for axis in ["x", "y"] {
            let (a, b) = (
                mirrored_part["pose"]["at"][axis].as_f64().unwrap(),
                shipped_part["pose"]["at"][axis].as_f64().unwrap(),
            );
            assert!((a - b).abs() < 1e-9, "{id} {axis}: {a} vs {b}");
        }
        assert!(
            angles_match(
                mirrored_part["pose"]["rotation"].as_f64().unwrap(),
                shipped_part["pose"]["rotation"].as_f64().unwrap(),
            ),
            "{id} rotation"
        );
    }
    // The source half is untouched by its own reflection.
    assert_same_switches(&switches(mirrored, "left"), &left, "corne source half");
    // The new instance is a pair member that resolves through the pair's construction.
    effective_inputs(
        &serde_json::from_value::<ProjectDoc>(mirrored.clone()).unwrap(),
        &[],
        &MechanicalTarget::Instance {
            instance_id: "right".into(),
        },
    )
    .unwrap();
}

#[test]
fn source_right_splits_reproduce_the_shipped_left_keys() {
    for name in ["lily58", "klor"] {
        let shipped = &projects()[name];
        let (mut engine, seed) = open(&half_only(shipped, "right"));
        let shipped_left = switches(shipped, "left");
        let right = switches(&seed, "right");
        let reply = create(
            &mut engine,
            &seed,
            "pair-left",
            "right",
            new_board("left", "left"),
            mapping(axis(&right, &shipped_left)),
        )
        .unwrap();
        assert_same_switches(&switches(&reply["document"], "left"), &shipped_left, name);
        assert_eq!(
            pair(&reply["document"], "pair-left")["constructionSource"],
            "right",
            "{name}"
        );
        assert_eq!(
            instance(&reply["document"], "left")["half"],
            "left",
            "{name}"
        );
        assert_eq!(
            instance(&reply["document"], "left")["role"],
            "central",
            "{name}"
        );
    }
}

#[test]
fn changed_ids_are_exactly_the_new_items_and_their_source_ids() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = open(&half_only(shipped, "left"));
    let shipped_right = switches(shipped, "right");
    let left = switches(&seed, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        mapping(axis(&left, &shipped_right)),
    )
    .unwrap();
    let document = &reply["document"];
    let record = &pair(document, "pair-right")["halves"];
    let mut expected: BTreeSet<String> = ["pair-right", "left", "right", "right"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    for correspondence in record["correspondences"].as_array().unwrap() {
        expected.insert(correspondence["sourceId"].as_str().unwrap().to_owned());
        expected.insert(correspondence["mirrorId"].as_str().unwrap().to_owned());
    }
    let changed: BTreeSet<String> = reply["scene"]["changedIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(changed, expected);
    assert!(changed.contains("matrix/right-keys/r0c0"));
    assert!(changed.contains("matrix/left-keys/r0c0"));
}

#[test]
fn one_undo_step_removes_the_half_and_redo_restores_it() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = open(&half_only(shipped, "left"));
    let before = snapshot(&mut engine);
    let shipped_right = switches(shipped, "right");
    let left = switches(&seed, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        mapping(axis(&left, &shipped_right)),
    )
    .unwrap();
    assert_eq!(
        reply["document"]["revision"].as_u64().unwrap(),
        before["revision"].as_u64().unwrap() + 1
    );
    let made_revision = reply["document"]["revision"].as_u64().unwrap();
    // Undo and redo are new revisions, so the restored document matches the one before the
    // edit in everything but its revision.
    let mut undone = request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
    assert_eq!(undone["revision"].as_u64().unwrap(), made_revision + 1);
    undone["revision"] = before["revision"].clone();
    assert_eq!(undone, before);
    let mut redone = request(&mut engine, json!({"id":"redo","kind":"redo"}))["document"].clone();
    assert_eq!(redone["revision"].as_u64().unwrap(), made_revision + 2);
    redone["revision"] = reply["document"]["revision"].clone();
    assert_eq!(redone, reply["document"]);
}

#[test]
fn the_produced_record_reopens_as_a_valid_document() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = open(&half_only(shipped, "left"));
    let shipped_right = switches(shipped, "right");
    let left = switches(&seed, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        mapping(axis(&left, &shipped_right)),
    )
    .unwrap();
    let (_, reopened) = open(&reply["document"]);
    assert_eq!(pair(&reopened, "pair-right")["constructionSource"], "left");
    assert_eq!(
        pair(&reopened, "pair-right")["halves"]["mirrorBoardId"],
        "right"
    );
}

#[test]
fn refused_requests_leave_the_document_unchanged() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = open(&half_only(shipped, "left"));
    let shipped_right = switches(shipped, "right");
    let left = switches(&seed, "left");
    let good_mapping = mapping(axis(&left, &shipped_right));
    let before = snapshot(&mut engine);

    // A new instance id that an existing instance already uses.
    let error = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "left"),
        good_mapping.clone(),
    )
    .unwrap_err();
    assert!(error.contains("left"), "{error}");
    assert_eq!(snapshot(&mut engine), before);

    // An unknown source instance.
    create(
        &mut engine,
        &seed,
        "pair-right",
        "nowhere",
        new_board("right", "right"),
        good_mapping.clone(),
    )
    .unwrap_err();
    assert_eq!(snapshot(&mut engine), before);

    // A new board id that the document already has.
    create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("left", "right"),
        good_mapping.clone(),
    )
    .unwrap_err();
    assert_eq!(snapshot(&mut engine), before);

    // A good creation, then a second half of the same source and a reused pair id.
    let made = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        good_mapping.clone(),
    )
    .unwrap();
    let after = snapshot(&mut engine);
    assert_eq!(after, made["document"]);
    create(
        &mut engine,
        &after,
        "pair-second",
        "left",
        new_board("second", "second"),
        good_mapping.clone(),
    )
    .unwrap_err();
    create(
        &mut engine,
        &after,
        "pair-right",
        "left",
        new_board("third", "third"),
        good_mapping,
    )
    .unwrap_err();
    assert_eq!(snapshot(&mut engine), after);
}

#[test]
fn reuse_board_is_a_flipped_instance_that_copies_nothing() {
    let shipped = &projects()["corne"];
    let (mut engine, seed) = open(&half_only(shipped, "left"));
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
    // Nothing is copied: boards, parts, matrices and layouts are the same as the seed's.
    assert_eq!(document["boards"], seed["boards"]);
    assert_eq!(document["parts"], seed["parts"]);
    assert_eq!(document["matrices"], seed["matrices"]);
    assert_eq!(document["layouts"], seed["layouts"]);
    // The flipped instance is what physical setup makes for a reused PCB.
    let flipped = instance(document, "flipped");
    assert_eq!(flipped["boardId"], "left");
    assert_eq!(flipped["half"], "right");
    assert_eq!(flipped["role"], "peripheral");
    assert_eq!(flipped["flipped"], true);
    assert_eq!(flipped["controllerPartId"], Value::Null);
    let record = pair(document, "pair-flipped");
    assert_eq!(record["members"], json!(["left", "flipped"]));
    assert_eq!(
        record["mapping"],
        json!({"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0})
    );
    assert_eq!(record["constructionSource"], "left");
    assert_eq!(record["halves"]["sourceBoardId"], "left");
    assert_eq!(record["halves"]["mirrorBoardId"], "left");
    // The reused board is one half of one record, so a second reuse of it is refused.
    let (_, reopened) = open(document);
    assert_eq!(pair(&reopened, "pair-flipped")["id"], "pair-flipped");
    let refusal = create(
        &mut engine,
        document,
        "pair-again",
        "left",
        json!({"kind":"reuse-board","instanceId":"again"}),
        mapping(0.0),
    )
    .unwrap_err();
    assert!(!refusal.is_empty());
    effective_inputs(
        &serde_json::from_value::<ProjectDoc>(document.clone()).unwrap(),
        &[],
        &MechanicalTarget::Instance {
            instance_id: "flipped".into(),
        },
    )
    .unwrap();
}
