#[path = "../../../mirrored-half/src/mirrored_half/inverse/json_comparison.rs"]
mod json_comparison;
use boardstudio_model::model::*;
use json_comparison::{assert_close, op};
use serde_json::{Value, json};

fn source_half() -> ProjectDoc {
    let switch = "generator:ceoloide/switch_mx";
    let diode = "generator:ceoloide/diode_tht_sod123";
    let mut cells = Vec::new();
    for row in 0..2 {
        for column in 0..3 {
            cells.push(json!({
                "row": row, "column": column, "enabled": true,
                "offset": {"x": f64::from(row) * 0.5, "y": f64::from(column) * -0.25},
                "rotation": f64::from(column) * 4.0,
                "assemblies": [
                    {"id": "diode", "definitionId": diode, "offset": {"x": 8, "y": -2},
                     "rotation": 90, "side": "back"},
                    {"id": "led", "definitionId": "synthetic/led", "offset": {"x": -3, "y": 4},
                     "rotation": 15, "side": "front"},
                    {"id": "stab", "definitionId": "kicad:marbastlib/STAB_MX_2u",
                     "offset": {"x": 0, "y": -12}, "rotation": 30, "side": "front"}]
            }));
        }
    }
    let mut part_ids: Vec<String> = Vec::new();
    for row in 0..2 {
        for column in 0..3 {
            part_ids.push(format!("matrix/left-keys/r{row}c{column}"));
        }
    }
    let definitions = json!([
            {"id": switch, "name": "Switch", "kind": "switch", "keycap": {"x": 18, "y": 18},
             "courtyard": [{"x":-9,"y":-9},{"x":9,"y":-9},{"x":9,"y":9},{"x":-9,"y":9}], "pads": [],
             "terminals": {"row": ["pad-1"], "column": ["pad-2"]},
             "matrixTerminals": {"row": "row", "column": "column"},
             "generator": {"source": "ceoloide/switch_mx", "version": "bundled-1",
                           "parameters": {"hotswap": true, "solder": false}}},
            {"id": diode, "name": "Diode", "kind": "passive",
             "courtyard": [{"x":-2,"y":-1},{"x":2,"y":-1},{"x":2,"y":1},{"x":-2,"y":1}], "pads": [],
             "generator": {"source": "ceoloide/diode_tht_sod123", "version": "bundled-1",
                           "parameters": {"side": "B"}}},
            {"id": "synthetic/led", "name": "Indicator", "kind": "passive",
             "courtyard": [{"x":-1.5,"y":-1.5},{"x":1.5,"y":-1.5},{"x":1.5,"y":1.5},{"x":-1.5,"y":1.5}],
             "pads": []},
            {"id": "kicad:marbastlib/STAB_MX_2u", "name": "Stabiliser", "kind": "passive",
             "courtyard": [{"x":-3,"y":-1},{"x":3,"y":-1},{"x":3,"y":1},{"x":-3,"y":1}],
             "pads": []},
            {"id": "synthetic/controller", "name": "Controller", "kind": "controller",
             "courtyard": [{"x":-5,"y":-4},{"x":5,"y":-4},{"x":5,"y":4},{"x":-5,"y":4}], "pads": []}
    ]);
    let parts = json!([
            {"id": "left/U1", "definitionId": "synthetic/controller", "reference": "U1",
             "pose": {"at": {"x": -24, "y": 5}, "rotation": 35}, "side": "front"},
            {"id": "left/U2", "definitionId": "synthetic/controller", "reference": "U2",
             "pose": {"at": {"x": -30, "y": -12}, "rotation": 180}, "side": "front"}
    ]);
    let matrices = json!([{"id": "left-keys", "name": "Keys", "rows": 2, "columns": 3,
            "pitch": {"x": 19, "y": 21}, "origin": {"x": 4, "y": 9}, "rotation": 12,
            "columnLayout": [{"splay": 0, "stagger": 0}, {"splay": 6, "stagger": 3}, {"splay": -3, "stagger": -2}],
"definitionId": switch, "boardId": "left", "partIds": [], "cells": cells}]);
    let hardware = json!({"topology": "split", "transport": "wired",
            "instances": [{"id": "left", "name": "Left", "boardId": "left", "half": "left",
                "role": "central", "flipped": false, "controllerPartId": "left/U1",
                "mechanical": {"boardId": "left", "method": "printed", "mount": "rigid",
                    "integratedPlateFrame": false, "bottomStyle": "shell", "plateThickness": 1.5,
                    "plateFoamThickness": 0, "pcbThickness": 1.6, "bottomFoamThickness": 0,
                    "batteryHeight": 0, "bottomThickness": 2, "plateToPcb": 3.5,
                    "wallThickness": 2, "clearance": 0.2, "profiles": [], "mounts": [],
                    "closureMounts": [], "openings": []}}],
            "boards": [{"boardId": "left", "controllerPartId": "left/U1", "mode": "matrix"}],
            "pairs": []});
    let document = json!({
        "format": "boardstudio/v2", "formatVersion": 5, "id": "round-trip", "name": "Round trip",
        "revision": 0, "parameters": {}, "definitions": definitions, "parts": parts,
        "matrices": matrices, "layouts": [], "nets": [], "outline": [], "caseBodies": [],
        "materials": [], "assets": [], "scripts": [], "constraints": [],
        "boards": [{"id": "left", "name": "Left", "outlineIds": [], "netIds": [],
            "thickness": 1.6, "partIds": ["left/U1", "left/U2"]}],
        "hardware": hardware});
    let mut document: ProjectDoc = serde_json::from_value(document).unwrap();
    let matrix = document.matrices[0].clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &matrix)
        .expect("the source matrix materialises");
    assert_eq!(document.matrices[0].part_ids.len(), part_ids.len() * 4);
    document
}

fn created(mapping: Value, companion_mode: &str, tweak: Option<Value>) -> ProjectDoc {
    let mut document = source_half();
    let mut request = json!({
        "kind": "create-mirrored-half", "pairId": "pair", "sourceInstanceId": "left",
        "mode": {"kind": "new-board", "boardId": "right", "instanceId": "right"},
        "mapping": mapping, "idRule": "board-prefix", "companionMode": companion_mode});
    if let Some(tweak) = tweak {
        request["companionTweak"] = tweak;
    }
    crate::support::apply_for_test(&mut document, &[], &op(request)).expect("the half is created");
    document
}

/// The mirror matrix of a created half, as a request, translated back to its source.
fn round_tripped(document: &ProjectDoc) -> (Value, Value) {
    let mirror = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "right-keys")
        .expect("the mirror matrix exists");
    let request = EditOperation::SetMatrix {
        matrix: mirror.clone(),
        definitions: None,
    };
    let result =
        crate::support::canonicalize(document, &request).expect("the mirror matrix translates");
    let EditOperation::SetMatrix { matrix, .. } = result.operation else {
        panic!("a matrix request is expected");
    };
    let stored = document
        .matrices
        .iter()
        .find(|candidate| candidate.id == "left-keys")
        .unwrap();
    (
        serde_json::to_value(&matrix).unwrap(),
        serde_json::to_value(stored).unwrap(),
    )
}

/// The same cell with `rotation` read modulo a turn and a missing companion turn read as none.
fn comparable(mut matrix: Value) -> Value {
    for cell in matrix["cells"].as_array_mut().into_iter().flatten() {
        for assembly in cell["assemblies"].as_array_mut().into_iter().flatten() {
            if assembly.get("rotation").is_none() {
                assembly["rotation"] = json!(0.0);
            }
        }
    }
    matrix
}

const MAPS: [(f64, f64, f64); 3] = [(200.0, 0.0, 0.0), (310.0, 42.0, 27.0), (150.0, -35.0, 90.0)];

#[test]
fn a_created_mirror_matrix_translates_back_to_its_stored_source_in_every_companion_mode() {
    for (x, y, rotation) in MAPS {
        let mapping = json!({"at": {"x": x, "y": y}, "rotation": rotation});
        for (mode, tweak) in [
            ("literal-mirror", None),
            ("duplicate-position", None),
            (
                "literal-mirror-with-tweak",
                Some(json!({"x": 0.75, "y": -0.5, "rotation": 8.0})),
            ),
        ] {
            let document = created(mapping.clone(), mode, tweak);
            let (translated, stored) = round_tripped(&document);
            assert_close(
                &comparable(translated),
                &comparable(stored),
                &format!("map ({x}, {y}, {rotation}) {mode}"),
            );
        }
    }
}

#[test]
fn every_created_mirror_part_moves_its_source_to_where_the_source_is_stored() {
    for (x, y, rotation) in MAPS {
        let mapping = json!({"at": {"x": x, "y": y}, "rotation": rotation});
        for (mode, tweak) in [
            ("literal-mirror", None),
            ("duplicate-position", None),
            (
                "literal-mirror-with-tweak",
                Some(json!({"x": 0.75, "y": -0.5, "rotation": 8.0})),
            ),
        ] {
            let document = created(mapping.clone(), mode, tweak);
            let halves = document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap();
            let mut checked = 0;
            for correspondence in halves
                .correspondences
                .iter()
                .filter(|correspondence| correspondence.kind == MirroredItemKind::Part)
            {
                let mirror = document
                    .parts
                    .iter()
                    .find(|part| part.id == correspondence.mirror_id)
                    .unwrap();
                let source = document
                    .parts
                    .iter()
                    .find(|part| part.id == correspondence.source_id)
                    .unwrap();
                let request = EditOperation::MoveParts {
                    positions: vec![boardstudio_model::model::Position {
                        id: mirror.id.clone(),
                        at: mirror.pose.at,
                    }],
                };
                let result = crate::support::canonicalize(&document, &request).unwrap();
                let EditOperation::MoveParts { positions } = result.operation else {
                    panic!("a move is expected");
                };
                let label = format!("map ({x}, {y}, {rotation}) {mode} {}", mirror.id);
                assert_eq!(positions[0].id, source.id, "{label}");
                assert_close(
                    &serde_json::to_value(positions[0].at).unwrap(),
                    &serde_json::to_value(source.pose.at).unwrap(),
                    &label,
                );
                checked += 1;
            }
            assert_eq!(checked, 6 * 4 + 2, "map ({x}, {y}, {rotation}) {mode}");
        }
    }
}
