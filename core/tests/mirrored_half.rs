//! `CreateMirroredHalf` (ticket 18) through Core's request surface using a small authored input.

use crate::support::synthetic_half;
use boardstudio_core::CoreEngine;
use boardstudio_core::mechanical::MechanicalTarget;
use boardstudio_core::mechanical::inputs::effective_inputs;
use boardstudio_core::model::ProjectDoc;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[path = "mirrored_half/activation_edges.rs"]
mod activation_edges;
#[path = "mirrored_half/circuit_child_acceptance.rs"]
mod circuit_child_acceptance;
#[path = "mirrored_half/edit_family_acceptance.rs"]
mod edit_family_acceptance;
#[path = "mirrored_half/matrix_acceptance.rs"]
mod matrix_acceptance;
#[path = "mirrored_half/mh_chunk0.rs"]
mod mh_chunk0;
#[path = "mirrored_half/mh_chunk1.rs"]
mod mh_chunk1;
#[path = "mirrored_half/mh_chunk2.rs"]
mod mh_chunk2;
#[path = "mirrored_half/mh_chunk3.rs"]
mod mh_chunk3;
#[path = "mirrored_half/noop_acceptance.rs"]
mod noop_acceptance;
#[path = "mirrored_half/open_acceptance.rs"]
mod open_acceptance;
#[path = "mirrored_half/parent_acceptance.rs"]
mod parent_acceptance;
#[path = "mirrored_half/replacement_pins.rs"]
mod replacement_pins;
#[path = "mirrored_half/sequence_invariants.rs"]
mod sequence_invariants;
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

fn constraint_by_id(document: &Value, id: &str) -> Value {
    document["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|constraint| constraint["id"] == id)
        .unwrap_or_else(|| panic!("constraint {id} is unavailable"))
        .clone()
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
        // Controllers keep the reflected heading without the half turn, so both halves
        // share the MCU heading; every other independent part takes the full rule.
        let definition_id = source["definitionId"].as_str().unwrap();
        let is_controller = document["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|definition| {
                definition["id"] == definition_id && definition["kind"] == "controller"
            });
        let expected_heading = if in_matrix || is_controller {
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
