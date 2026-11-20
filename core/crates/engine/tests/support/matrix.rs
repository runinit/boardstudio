use boardstudio_core_mirrored_half::mirrored_half::{
    inverse::{Canonical, MatrixAddition},
    matrix_addition::*,
};
use boardstudio_model::model::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;
pub const SWITCH: &str = "generator:ceoloide/switch_mx";
pub const DIODE: &str = "generator:ceoloide/diode_tht_sod123";
pub const STABILISER: &str = "kicad:marbastlib/STAB_MX_2u";

pub fn op(value: Value) -> EditOperation {
    serde_json::from_value(value).expect("the operation is valid")
}

/// A left half with one 2 x 2 matrix (`left-keys`, a diode on each key) and a controller.
pub fn source_half() -> ProjectDoc {
    let mut cells = Vec::new();
    for row in 0..2 {
        for column in 0..2 {
            cells.push(json!({
                "row": row, "column": column, "enabled": true,
                "assemblies": [{"id": "diode", "definitionId": DIODE,
                    "offset": {"x": 8, "y": -2}, "rotation": 90, "side": "back"}]
            }));
        }
    }
    let definitions = json!([
        {"id": SWITCH, "name": "Switch", "kind": "switch", "keycap": {"x": 18, "y": 18},
         "courtyard": [{"x":-9,"y":-9},{"x":9,"y":-9},{"x":9,"y":9},{"x":-9,"y":9}], "pads": [],
         "terminals": {"row": ["pad-1"], "column": ["pad-2"]},
         "matrixTerminals": {"row": "row", "column": "column"},
         "generator": {"source": "ceoloide/switch_mx", "version": "bundled-1",
                       "parameters": {"hotswap": true, "solder": false}}},
        {"id": DIODE, "name": "Diode", "kind": "passive",
         "courtyard": [{"x":-2,"y":-1},{"x":2,"y":-1},{"x":2,"y":1},{"x":-2,"y":1}], "pads": [],
         "generator": {"source": "ceoloide/diode_tht_sod123", "version": "bundled-1",
                       "parameters": {"side": "B"}}},
        {"id": STABILISER, "name": "Stabiliser", "kind": "passive",
         "courtyard": [{"x":-3,"y":-1},{"x":3,"y":-1},{"x":3,"y":1},{"x":-3,"y":1}], "pads": []},
        {"id": "synthetic/controller", "name": "Controller", "kind": "controller",
         "courtyard": [{"x":-5,"y":-4},{"x":5,"y":-4},{"x":5,"y":4},{"x":-5,"y":4}],
         "pads": [
            {"id":"P1","number":"P1","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"},
            {"id":"P2","number":"P2","at":{"x":2,"y":0},"size":{"x":1,"y":1},"shape":"circle"}
         ]}
    ]);
    let parts = json!([
        {"id": "left/U1", "definitionId": "synthetic/controller", "reference": "U1",
         "pose": {"at": {"x": -24, "y": 5}, "rotation": 35}, "side": "front"}
    ]);
    let matrices = json!([{"id": "left-keys", "name": "Keys", "rows": 2, "columns": 2,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 4, "y": 9},
        "definitionId": SWITCH, "boardId": "left", "partIds": [], "cells": cells}]);
    let mechanical = json!({"boardId": "left", "method": "printed", "mount": "rigid",
        "integratedPlateFrame": false, "bottomStyle": "shell", "plateThickness": 1.5,
        "plateFoamThickness": 0, "pcbThickness": 1.6, "bottomFoamThickness": 0,
        "batteryHeight": 0, "bottomThickness": 2, "plateToPcb": 3.5,
        "wallThickness": 2, "clearance": 0.2, "profiles": [], "mounts": [],
        "closureMounts": [], "openings": []});
    let hardware = json!({"topology": "split", "transport": "wired",
        "instances": [{"id": "left", "name": "Left", "boardId": "left", "half": "left",
            "role": "central", "flipped": false, "controllerPartId": "left/U1",
            "mechanical": mechanical}],
        "boards": [{"boardId": "left", "controllerPartId": "left/U1", "mode": "matrix"}],
        "pairs": []});
    let document = json!({
        "format": "boardstudio/v2", "formatVersion": 5, "id": "forward", "name": "Forward",
        "revision": 0, "parameters": {}, "definitions": definitions, "parts": parts,
        "matrices": matrices, "layouts": [], "nets": [], "outline": [], "caseBodies": [],
        "materials": [], "assets": [], "scripts": [], "constraints": [],
        "boards": [{"id": "left", "name": "Left", "outlineIds": [], "netIds": [],
            "thickness": 1.6, "partIds": ["left/U1"]}],
        "hardware": hardware});
    let mut document: ProjectDoc = serde_json::from_value(document).unwrap();
    let matrix = document.matrices[0].clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &matrix)
        .expect("the source matrix materialises");
    document
}

pub fn created(mapping: Value, companion_mode: &str) -> ProjectDoc {
    create_from(source_half(), mapping, companion_mode, None)
}

pub fn create_from(
    mut document: ProjectDoc,
    mapping: Value,
    companion_mode: &str,
    tweak: Option<Value>,
) -> ProjectDoc {
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

pub fn pair_document() -> ProjectDoc {
    created(
        json!({"at": {"x": 200, "y": 0}, "rotation": 0}),
        "literal-mirror",
    )
}

pub fn new_matrix(id: &str, board: &str, origin: (f64, f64), columns: u32) -> Value {
    let mirror_side = board == "right";
    let sign = if mirror_side { -1 } else { 1 };
    // Every slot is stored, as the editor stores a matrix.
    let mut cells = vec![
        json!({"row": 0, "column": 0, "enabled": true, "assemblies": [
        {"id": "diode", "definitionId": DIODE,
         "offset": {"x": 8 * sign, "y": -2}, "rotation": 90 * sign, "side": "back"}]}),
    ];
    cells.extend((1..columns).map(|column| json!({"row": 0, "column": column, "enabled": true})));
    let mut matrix = json!({
        "id": id, "name": "Cluster", "rows": 1, "columns": columns,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": origin.0, "y": origin.1},
        "definitionId": SWITCH, "boardId": board, "partIds": [], "cells": cells
    });
    if mirror_side {
        matrix["mirror"] = json!("x");
    }
    json!({"kind": "set-matrix", "matrix": matrix})
}

/// What 111 will do: canonicalise, apply the source request, then run each forward transition.
pub fn canonical_and_source(document: &ProjectDoc, request: &Value) -> (Canonical, ProjectDoc) {
    let canonical = crate::support::canonicalize(document, &op(request.clone()))
        .unwrap_or_else(|refusal| panic!("{refusal}"));
    let mut next = document.clone();
    crate::support::apply_for_test(&mut next, &[], &canonical.operation)
        .expect("the source operation applies");
    (canonical, next)
}

pub fn add_matrix(document: &ProjectDoc, request: &Value) -> ProjectDoc {
    let (canonical, mut next) = canonical_and_source(document, request);
    assert_eq!(canonical.matrix_additions.len(), 1, "one planned pair");
    for addition in &canonical.matrix_additions {
        next = mirror_added_matrix(&next, addition)
            .unwrap_or_else(|error| panic!("{error}"))
            .draft;
    }
    next
}

pub fn plan(document: &ProjectDoc, request: &Value) -> (MatrixAddition, ProjectDoc) {
    let (canonical, next) = canonical_and_source(document, request);
    (canonical.matrix_additions[0].clone(), next)
}

pub fn refusal(document: &ProjectDoc, addition: &MatrixAddition) -> String {
    mirror_added_matrix(document, addition)
        .err()
        .expect("the transition is refused")
}

pub fn halves(document: &ProjectDoc) -> &boardstudio_model::model::MirroredHalves {
    document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
}

pub fn correspondence(
    document: &ProjectDoc,
    kind: MirroredItemKind,
    source: &str,
) -> Option<String> {
    halves(document)
        .correspondences
        .iter()
        .find(|item| item.kind == kind && item.source_id == source)
        .map(|item| item.mirror_id.clone())
}

pub fn matrix<'a>(document: &'a ProjectDoc, id: &str) -> &'a boardstudio_model::model::Matrix {
    document
        .matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .unwrap_or_else(|| panic!("matrix {id} exists"))
}

/// Every member sits once in the parts, once on its board and once in its matrix, and has exactly
/// one correspondence on each side.
pub fn assert_members_exactly_once(document: &ProjectDoc, addition: &MatrixAddition) {
    let source = matrix(document, &addition.source_id);
    let mirror = matrix(document, &addition.mirror_id);
    assert_eq!(
        source.board_id.as_deref(),
        Some(addition.source_board_id.as_str())
    );
    assert_eq!(
        mirror.board_id.as_deref(),
        Some(addition.mirror_board_id.as_str())
    );
    assert_eq!(source.part_ids.len(), mirror.part_ids.len());
    for (source_id, mirror_id) in &addition.members {
        for (id, board, owner) in [
            (source_id, &addition.source_board_id, source),
            (mirror_id, &addition.mirror_board_id, mirror),
        ] {
            assert_eq!(
                document.parts.iter().filter(|p| &p.id == id).count(),
                1,
                "{id}"
            );
            assert_eq!(
                document
                    .boards
                    .iter()
                    .map(|b| b.part_ids.iter().filter(|p| *p == id).count())
                    .sum::<usize>(),
                1,
                "{id} is registered once"
            );
            assert!(
                document
                    .boards
                    .iter()
                    .find(|b| &b.id == board)
                    .unwrap()
                    .part_ids
                    .contains(id),
                "{id} is on {board}"
            );
            assert_eq!(
                owner.part_ids.iter().filter(|p| *p == id).count(),
                1,
                "{id}"
            );
        }
        let links = halves(document)
            .correspondences
            .iter()
            .filter(|c| c.kind == MirroredItemKind::Part)
            .filter(|c| &c.source_id == source_id || &c.mirror_id == mirror_id)
            .count();
        assert_eq!(links, 1, "{source_id} <-> {mirror_id}");
        assert_eq!(
            correspondence(document, MirroredItemKind::Part, source_id).as_deref(),
            Some(mirror_id.as_str())
        );
    }
}

pub fn parts_json(document: &ProjectDoc, ids: &BTreeSet<String>) -> Vec<Value> {
    document
        .parts
        .iter()
        .filter(|part| ids.contains(&part.id))
        .map(|part| serde_json::to_value(part).unwrap())
        .collect()
}
