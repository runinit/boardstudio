//! Direct translation tests for the linked-mirror inverse seam (ticket 110).
//!
//! Every expectation is a literal worked from the pair map by hand: the axis map sends
//! `(x, y)` to `(200 - x, y)`, and the rotated map `(100, -30, 90 degrees)` sends the source
//! point `(a, b)` to `(100 - b, -30 - a)`. No expectation calls the production mapping.

use super::*;
#[path = "json_comparison.rs"]
mod json_comparison;
use crate::model::{Constraint, EditOperation, Pose2, Vec2};
use json_comparison::{assert_close, op};
use serde_json::{Value, json};

#[path = "tests/inv_chunk0.rs"]
mod inv_chunk0;
#[path = "tests/inv_chunk1.rs"]
mod inv_chunk1;
#[path = "tests/inv_chunk2.rs"]
mod inv_chunk2;
const ROTATED: Pose2 = Pose2 {
    at: Vec2 { x: 100.0, y: -30.0 },
    rotation: 90.0,
};

fn part(id: &str, definition: &str, reference: &str, x: f64, y: f64, rotation: f64) -> Value {
    json!({"id": id, "definitionId": definition, "reference": reference,
        "pose": {"at": {"x": x, "y": y}, "rotation": rotation}, "side": "front"})
}

fn corr(kind: &str, source: &str, mirror: &str) -> Value {
    json!({"kind": kind, "sourceId": source, "mirrorId": mirror})
}

/// A linked pair: matrix `left-keys` (1 x 2, a diode on its first key) on board `left`, its
/// reflection `right-keys` on board `right`, one independent part each side, a Local mirror
/// part, an Omitted mirror part and an unpaired mirror-board part.
fn fixture() -> ProjectDoc {
    let source_cells = json!([
        {"row": 0, "column": 0, "enabled": true, "assemblies": [
            {"id": "diode", "definitionId": "d", "offset": {"x": 8, "y": -2}, "rotation": 90}]},
        {"row": 0, "column": 1, "enabled": true}
    ]);
    let mirror_cells = json!([
        {"row": 0, "column": 0, "enabled": true, "assemblies": [
            {"id": "diode", "definitionId": "d", "offset": {"x": -8, "y": -2}, "rotation": -90}]},
        {"row": 0, "column": 1, "enabled": true}
    ]);
    let document = json!({
        "format": "boardstudio/v2", "formatVersion": 5, "id": "inverse", "name": "Inverse",
        "revision": 0, "parameters": {},
        "definitions": [], "layouts": [], "nets": [], "outline": [], "caseBodies": [],
        "materials": [], "assets": [], "scripts": [], "constraints": [],
        "boards": [
            {"id": "left", "name": "left", "outlineIds": [], "netIds": [], "thickness": 1.6,
             "partIds": ["matrix/left-keys/r0c0", "matrix/left-keys/r0c0/diode",
                         "matrix/left-keys/r0c1", "left/U1", "left/HEADER"]},
            {"id": "right", "name": "right", "outlineIds": [], "netIds": [], "thickness": 1.6,
             "partIds": ["matrix/right-keys/r0c0", "matrix/right-keys/r0c0/diode",
                         "matrix/right-keys/r0c1", "right/U1", "right/HEADER", "right/FREE"]}
        ],
        "parts": [
            part("matrix/left-keys/r0c0", "sw", "SW1", 0.0, 0.0, 0.0),
            part("matrix/left-keys/r0c0/diode", "d", "D1", 8.0, -2.0, 90.0),
            part("matrix/left-keys/r0c1", "sw", "SW2", 19.0, 0.0, 0.0),
            part("left/U1", "u", "U1", -24.0, 0.0, 0.0),
            part("left/HEADER", "u", "J1", -24.0, 30.0, 0.0),
            part("matrix/right-keys/r0c0", "sw", "RSW1", 200.0, 0.0, 0.0),
            part("matrix/right-keys/r0c0/diode", "d", "RD1", 192.0, -2.0, -90.0),
            part("matrix/right-keys/r0c1", "sw", "RSW2", 181.0, 0.0, 0.0),
            part("right/U1", "u", "RU1", 224.0, 0.0, 180.0),
            part("right/HEADER", "u", "RJ1", 224.0, 30.0, 180.0),
            part("right/FREE", "u", "RFREE", 300.0, 0.0, 0.0)
        ],
        "matrices": [
            {"id": "left-keys", "name": "Left keys", "rows": 1, "columns": 2,
             "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": 0}, "definitionId": "sw",
             "boardId": "left", "cells": source_cells,
             "partIds": ["matrix/left-keys/r0c0", "matrix/left-keys/r0c0/diode",
                         "matrix/left-keys/r0c1"]},
            {"id": "right-keys", "name": "Right keys", "rows": 1, "columns": 2,
             "pitch": {"x": 19, "y": 19}, "origin": {"x": 200, "y": 0}, "definitionId": "sw",
             "boardId": "right", "mirror": "x", "cells": mirror_cells,
             "partIds": ["matrix/right-keys/r0c0", "matrix/right-keys/r0c0/diode",
                         "matrix/right-keys/r0c1"]}
        ],
        "hardware": {"topology": "split", "transport": "wired", "instances": [], "boards": [],
            "pairs": [{
                "id": "pair", "members": ["left", "right"],
                "mapping": {"at": {"x": 200, "y": 0}, "rotation": 0},
                "construction": {"method": "printed", "mount": "rigid",
                    "integratedPlateFrame": false, "plateThickness": 1.5,
                    "plateFoamThickness": 0, "bottomFoamThickness": 0, "bottomThickness": 2,
                    "plateToPcb": 3.5, "wallThickness": 2, "clearance": 0.2},
                "halves": {
                    "sourceBoardId": "left", "mirrorBoardId": "right",
                    "correspondences": [
                        corr("matrix", "left-keys", "right-keys"),
                        corr("part", "matrix/left-keys/r0c0", "matrix/right-keys/r0c0"),
                        corr("part", "matrix/left-keys/r0c0/diode", "matrix/right-keys/r0c0/diode"),
                        corr("part", "matrix/left-keys/r0c1", "matrix/right-keys/r0c1"),
                        corr("part", "left/U1", "right/U1"),
                        corr("part", "left/HEADER", "right/HEADER"),
                        corr("part", "left/GONE", "right/GONE")
                    ],
                    "exceptions": [
                        {"kind": "part", "mirrorId": "right/HEADER", "state": "local"},
                        {"kind": "part", "mirrorId": "right/GONE", "state": "omitted"}
                    ]
                }
            }]}
    });
    serde_json::from_value(document).expect("the fixture is a valid project")
}

fn rotated_fixture() -> ProjectDoc {
    let mut document = fixture();
    document.hardware.as_mut().unwrap().pairs[0].mapping = ROTATED;
    document
}

fn native_layout_fixture() -> ProjectDoc {
    let mut document = fixture();
    let mut omitted_source = document
        .parts
        .iter()
        .find(|part| part.id == "left/U1")
        .unwrap()
        .clone();
    omitted_source.id = "left/GONE".into();
    omitted_source.reference = "HIDDEN".into();
    document.parts.push(omitted_source);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "left")
        .unwrap()
        .part_ids
        .push("left/GONE".into());
    document.layouts = vec![
        serde_json::from_value(json!({"id":"left-layout","name":"Source layout",
            "boardId":"left","matrixIds": ["left-keys"],
            "partIds":["left/U1","left/GONE","left/HEADER"]}))
        .unwrap(),
        serde_json::from_value(json!({"id":"right-layout","name":"Mirror layout",
            "boardId":"right","matrixIds": ["right-keys"],
            "partIds":["right/U1","right/HEADER","right/FREE"]}))
        .unwrap(),
    ];
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(
            serde_json::from_value(json!({"kind":"layout",
            "sourceId":"left-layout","mirrorId":"right-layout"}))
            .unwrap(),
        );
    document
}

fn add_empty_local_matrix(document: &mut ProjectDoc, id: &str) {
    document.matrices.push(
        serde_json::from_value(json!({"id":id,"name":id,"rows":0,"columns":0,
            "pitch":{"x":19.0,"y":19.0},"origin":{"x":0.0,"y":0.0},
            "definitionId":"sw","boardId":"right","partIds":[]}))
        .unwrap(),
    );
}

fn canonical(document: &ProjectDoc, operation: &Value) -> Result<Canonical, String> {
    canonicalize(document, &op(operation.clone())).map_err(|refusal| refusal.to_string())
}

fn expect_operation(document: &ProjectDoc, request: Value, expected: Value) {
    let result = canonical(document, &request).expect("the request translates");
    assert_close(
        &serde_json::to_value(&result.operation).unwrap(),
        &expected,
        "operation",
    );
}

fn refusal(document: &ProjectDoc, request: Value) -> String {
    canonical(document, &request).expect_err("the request is refused")
}

// ---- coverage ------------------------------------------------------------------------------

// ---- unrelated and local targets -----------------------------------------------------------

// ---- MoveParts -----------------------------------------------------------------------------

// ---- AddPart -------------------------------------------------------------------------------

// ---- RemoveParts ---------------------------------------------------------------------------

// ---- companions by placement mode ------------------------------------------------------------

const DIODE: &str = "matrix/right-keys/r0c0/diode";

fn set_companion(document: &mut ProjectDoc, mode: &str, tweak: Value) {
    let pair = &mut document.hardware.as_mut().unwrap().pairs[0];
    let halves = pair.halves.as_mut().unwrap();
    let correspondence = halves
        .correspondences
        .iter_mut()
        .find(|correspondence| correspondence.mirror_id == DIODE)
        .unwrap();
    correspondence.companion_mode = Some(serde_json::from_value(json!(mode)).unwrap());
    correspondence.companion_tweak = serde_json::from_value(tweak).unwrap();
}

fn pose_of<'a>(document: &'a mut ProjectDoc, id: &str) -> &'a mut Pose2 {
    &mut document
        .parts
        .iter_mut()
        .find(|part| part.id == id)
        .unwrap()
        .pose
}

fn move_diode(at: (f64, f64)) -> Value {
    json!({"kind": "move-parts", "positions": [{"id": DIODE, "at": {"x": at.0, "y": at.1}}]})
}

fn moved_source_diode(at: (f64, f64)) -> Value {
    json!({"kind": "move-parts", "positions": [
        {"id": "matrix/left-keys/r0c0/diode", "at": {"x": at.0, "y": at.1}}]})
}

// ---- SetMatrix -------------------------------------------------------------------------------

/// What the Inspector would send for the mirror matrix of the fixture, with `edits` merged over
/// its stored values.
fn mirror_payload(edits: Value) -> Value {
    let mut payload = json!({
    "id": "right-keys", "name": "Right keys", "rows": 1, "columns": 2,
    "pitch": {"x": 19, "y": 19}, "origin": {"x": 200, "y": 0}, "definitionId": "sw",
    "boardId": "right", "mirror": "x", "partIds": [],
    "cells": [
        {"row": 0, "column": 0, "enabled": true, "assemblies": [
            {"id": "diode", "definitionId": "d", "offset": {"x": -8, "y": -2}, "rotation": -90}]},
        {"row": 0, "column": 1, "enabled": true}
    ]});
    for (key, value) in edits.as_object().unwrap() {
        payload[key] = value.clone();
    }
    payload
}

fn set_matrix(payload: Value) -> Value {
    json!({"kind": "set-matrix", "matrix": payload})
}

fn translated_matrix(document: &ProjectDoc, payload: Value) -> Value {
    let result = canonical(document, &set_matrix(payload)).expect("the matrix translates");
    let value = serde_json::to_value(&result.operation).unwrap();
    assert_eq!(value["kind"], "set-matrix");
    value["matrix"].clone()
}

fn companion_matrix(document: &ProjectDoc, assembly_definition: &str, rotation: f64) -> Value {
    translated_matrix(
        document,
        mirror_payload(json!({
        "columnLayout": [{"splay": 0}, {"splay": 15}],
        "cells": [
            {"row": 0, "column": 0, "enabled": true, "rotation": 20, "assemblies": [
                {"id": "diode", "definitionId": assembly_definition,
                 "offset": {"x": -8, "y": -3}, "rotation": rotation}]},
            {"row": 0, "column": 1, "enabled": true, "assemblies": [
                {"id": "diode", "definitionId": assembly_definition,
                 "offset": {"x": -8, "y": -3}, "rotation": 10}]}
        ]})),
    )
}

// ---- a new matrix on a paired board is mirrored and linked ------------------------------------

fn new_matrix(id: &str, board: &str, origin: (f64, f64)) -> Value {
    let mut request = set_matrix(json!({
        "id": id, "name": "Thumbs", "rows": 1, "columns": 2, "pitch": {"x": 19, "y": 19},
        "origin": {"x": origin.0, "y": origin.1}, "definitionId": "sw", "boardId": board,
        "partIds": [],
        "cells": [{"row": 0, "column": 0, "enabled": true, "assemblies": [
            {"id": "diode", "definitionId": "d",
             "offset": {"x": if board == "right" { -8 } else { 8 }, "y": -2},
             "rotation": if board == "right" { -90 } else { 90 }}]}]
    }));
    if board == "right" {
        request["matrix"]["mirror"] = json!("x");
    }
    request
}

fn addition(result: &Canonical) -> &MatrixAddition {
    assert_eq!(result.matrix_additions.len(), 1, "one planned matrix pair");
    &result.matrix_additions[0]
}

// ---- matrix parameters and structure ---------------------------------------------------------

fn with_legacy_link(mut document: ProjectDoc) -> ProjectDoc {
    document.layouts = serde_json::from_value(json!([
        {"id": "legacy-a", "name": "A", "boardId": "left", "matrixIds": ["left-keys"],
         "partIds": [], "mirrorLink": {"sourceId": "legacy-b", "axisX": 0}},
        {"id": "legacy-b", "name": "B", "boardId": "left", "matrixIds": ["right-keys"], "partIds": []}
    ]))
    .unwrap();
    document
}

fn with_matrix_state(mut document: ProjectDoc, state: &str) -> ProjectDoc {
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.exceptions.push(
        serde_json::from_value(json!({"kind": "matrix", "mirrorId": "right-keys", "state": state}))
            .unwrap(),
    );
    document
}
