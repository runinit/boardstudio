//! Tests of the forward matrix-addition transition (ticket 145).
//!
//! The documents are built through the production creation path (`CreateMirroredHalf`), then a
//! matrix is added exactly as 111 will: `canonicalize` the request, apply the source-equivalent
//! operation, and hand the planned `MatrixAddition` to `mirror_added_matrix`.

use super::*;
use crate::mirrored_half::inverse::{self, Canonical};
use crate::model::{EditOperation, MirroredItemKind};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const SWITCH: &str = "generator:ceoloide/switch_mx";
const DIODE: &str = "generator:ceoloide/diode_tht_sod123";
const STABILISER: &str = "kicad:marbastlib/STAB_MX_2u";

pub(in crate::mirrored_half) fn op(value: Value) -> EditOperation {
    serde_json::from_value(value).expect("the operation is valid")
}

/// A left half with one 2 x 2 matrix (`left-keys`, a diode on each key) and a controller.
pub(in crate::mirrored_half) fn source_half() -> ProjectDoc {
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
    crate::matrix::set_matrix(&mut document, &matrix).expect("the source matrix materialises");
    document
}

fn created(mapping: Value, companion_mode: &str) -> ProjectDoc {
    create_from(source_half(), mapping, companion_mode, None)
}

pub(in crate::mirrored_half) fn create_from(
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
    crate::apply(&mut document, &[], &op(request)).expect("the half is created");
    document
}

pub(in crate::mirrored_half) fn pair_document() -> ProjectDoc {
    created(
        json!({"at": {"x": 200, "y": 0}, "rotation": 0}),
        "literal-mirror",
    )
}

pub(in crate::mirrored_half) fn new_matrix(
    id: &str,
    board: &str,
    origin: (f64, f64),
    columns: u32,
) -> Value {
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
fn canonical_and_source(document: &ProjectDoc, request: &Value) -> (Canonical, ProjectDoc) {
    let canonical = inverse::canonicalize(document, &op(request.clone()))
        .unwrap_or_else(|refusal| panic!("{refusal}"));
    let mut next = document.clone();
    crate::apply(&mut next, &[], &canonical.operation).expect("the source operation applies");
    (canonical, next)
}

pub(in crate::mirrored_half) fn add_matrix(document: &ProjectDoc, request: &Value) -> ProjectDoc {
    let (canonical, mut next) = canonical_and_source(document, request);
    assert_eq!(canonical.matrix_additions.len(), 1, "one planned pair");
    for addition in &canonical.matrix_additions {
        next = mirror_added_matrix(&next, addition)
            .unwrap_or_else(|error| panic!("{error}"))
            .draft;
    }
    next
}

fn plan(document: &ProjectDoc, request: &Value) -> (MatrixAddition, ProjectDoc) {
    let (canonical, next) = canonical_and_source(document, request);
    (canonical.matrix_additions[0].clone(), next)
}

fn refusal(document: &ProjectDoc, addition: &MatrixAddition) -> String {
    mirror_added_matrix(document, addition)
        .err()
        .expect("the transition is refused")
}

pub(in crate::mirrored_half) fn halves(document: &ProjectDoc) -> &crate::model::MirroredHalves {
    document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
}

fn correspondence(document: &ProjectDoc, kind: MirroredItemKind, source: &str) -> Option<String> {
    halves(document)
        .correspondences
        .iter()
        .find(|item| item.kind == kind && item.source_id == source)
        .map(|item| item.mirror_id.clone())
}

pub(in crate::mirrored_half) fn matrix<'a>(
    document: &'a ProjectDoc,
    id: &str,
) -> &'a crate::model::Matrix {
    document
        .matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .unwrap_or_else(|| panic!("matrix {id} exists"))
}

/// Every member sits once in the parts, once on its board and once in its matrix, and has exactly
/// one correspondence on each side.
fn assert_members_exactly_once(document: &ProjectDoc, addition: &MatrixAddition) {
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

fn parts_json(document: &ProjectDoc, ids: &BTreeSet<String>) -> Vec<Value> {
    document
        .parts
        .iter()
        .filter(|part| ids.contains(&part.id))
        .map(|part| serde_json::to_value(part).unwrap())
        .collect()
}

#[test]
fn a_base_matrix_then_a_thumb_cluster_on_each_half_are_mirrored_and_linked() {
    let before = pair_document();
    let untouched: BTreeSet<String> = before.parts.iter().map(|part| part.id.clone()).collect();
    let untouched_json = parts_json(&before, &untouched);
    let untouched_matrices = serde_json::to_value(&before.matrices).unwrap();

    // 1. A base matrix on the source half.
    let (base, with_base) = plan(&before, &new_matrix("left-base", "left", (10.0, 80.0), 3));
    let mut document = mirror_added_matrix(&with_base, &base).unwrap().draft;
    assert_eq!(base.mirror_id, "right-base");

    // 2. A thumb cluster on each half: the source half, then the mirror half.
    let (left, with_left) = plan(
        &document,
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    document = mirror_added_matrix(&with_left, &left).unwrap().draft;
    let (right, with_right) = plan(
        &document,
        &new_matrix("right-arc", "right", (150.0, 160.0), 2),
    );
    assert_eq!(right.source_id, "left-arc");
    assert_eq!(right.mirror_id, "right-arc");
    document = mirror_added_matrix(&with_right, &right).unwrap().draft;

    for addition in [&base, &left, &right] {
        assert_eq!(
            correspondence(&document, MirroredItemKind::Matrix, &addition.source_id).as_deref(),
            Some(addition.mirror_id.as_str()),
            "{} is linked",
            addition.source_id
        );
        assert!(!addition.members.is_empty());
        assert_members_exactly_once(&document, addition);
    }

    // Unrelated geometry is unchanged.
    assert_eq!(parts_json(&document, &untouched), untouched_json);
    let kept: Vec<_> = document
        .matrices
        .iter()
        .filter(|matrix| before.matrices.iter().any(|old| old.id == matrix.id))
        .collect();
    assert_eq!(serde_json::to_value(kept).unwrap(), untouched_matrices);

    // The record is valid as a whole, and the new matrices are reflections of their sources.
    crate::mechanical::halves::validate_document(&document).unwrap();
    let source = matrix(&document, "left-base");
    let mirror = matrix(&document, "right-base");
    assert_eq!((mirror.rows, mirror.columns), (source.rows, source.columns));
    assert_eq!(mirror.mirror, Some(crate::model::Mirror::X));
    assert!((mirror.origin.x - (200.0 - source.origin.x)).abs() < 1e-9);
    assert!((mirror.origin.y - source.origin.y).abs() < 1e-9);
}

#[test]
fn editing_a_new_matrix_from_the_mirror_half_names_its_source() {
    let document = add_matrix(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    let mirror_key = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/right-thumbs/r0c0")
        .expect("the mirror key exists");
    let request = EditOperation::MoveParts {
        positions: vec![crate::model::Position {
            id: mirror_key.id.clone(),
            at: crate::model::Vec2 {
                x: mirror_key.pose.at.x + 3.0,
                y: mirror_key.pose.at.y,
            },
        }],
    };
    let canonical = inverse::canonicalize(&document, &request).unwrap();
    let EditOperation::MoveParts { positions } = canonical.operation else {
        panic!("a move is expected");
    };
    assert_eq!(positions[0].id, "matrix/left-thumbs/r0c0");
    let source = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/left-thumbs/r0c0")
        .unwrap();
    assert!((positions[0].at.x - (source.pose.at.x - 3.0)).abs() < 1e-9);
}

#[test]
fn a_planned_mirror_matrix_that_has_become_taken_is_refused() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    let mut other = matrix(&next, "left-keys").clone();
    other.id = addition.mirror_id.clone();
    other.board_id = Some("right".into());
    other.part_ids.clear();
    other.cells.clear();
    next.matrices.push(other);
    let message = refusal(&next, &addition);
    assert!(message.contains("right-thumbs"), "{message}");
}

#[test]
fn a_planned_member_identity_that_has_become_taken_is_refused() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    let mut intruder = next.parts[0].clone();
    intruder.id = addition.members[0].1.clone();
    intruder.reference = "ZZ1".into();
    next.parts.push(intruder);
    let message = refusal(&next, &addition);
    assert!(message.contains("planned mirror member"), "{message}");
    assert!(message.contains(&addition.members[0].1), "{message}");
}

#[test]
fn a_matrix_already_named_by_a_correspondence_is_refused() {
    let document = add_matrix(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    let again = MatrixAddition {
        source_id: "left-thumbs".into(),
        mirror_id: "right-thumbs".into(),
        source_board_id: "left".into(),
        mirror_board_id: "right".into(),
        members: vec![],
    };
    let message = refusal(&document, &again);
    assert!(message.contains("right-thumbs"), "{message}");

    // A stale record: the correspondence survives although the matrices are gone.
    let mut stale = document.clone();
    stale
        .matrices
        .retain(|matrix| matrix.id != "right-thumbs" && matrix.id != "left-thumbs");
    let message = refusal(&stale, &again);
    assert!(
        message.contains("correspondence") || message.contains("unavailable"),
        "{message}"
    );
}

#[test]
fn added_matrix_electrical_functions_follow_the_new_member_correspondences() {
    let matrix_request = new_matrix("left-thumbs", "left", (10.0, 120.0), 2);
    let (addition, mut next) = plan(&pair_document(), &matrix_request);
    let source = next
        .hardware
        .as_mut()
        .unwrap()
        .boards
        .iter_mut()
        .find(|board| board.board_id == "left")
        .unwrap();
    source
        .assignments
        .insert("matrix/left-thumbs/r0c0/row".into(), "P1".into());
    source
        .locks
        .insert("matrix/left-thumbs/r0c1/column".into(), "P2".into());
    source
        .assignments
        .insert("unrelated/function".into(), "P1".into());
    source.locks.insert("unrelated/lock".into(), "P2".into());
    let mirror_config = next
        .hardware
        .as_mut()
        .unwrap()
        .boards
        .iter_mut()
        .find(|board| board.board_id == "right")
        .unwrap();
    mirror_config
        .assignments
        .insert("mirror-only/function".into(), "P2".into());
    mirror_config
        .locks
        .insert("mirror-only/lock".into(), "P1".into());
    let mirror_before = next
        .hardware
        .as_ref()
        .unwrap()
        .boards
        .iter()
        .find(|board| board.board_id == "right")
        .unwrap()
        .clone();

    // Independent oracle: when this exact matrix and wiring exist before pair creation, the
    // established CreateMirroredHalf path maps these assignments and locks to the same functions.
    let mut source_before_creation = source_half();
    crate::apply(&mut source_before_creation, &[], &op(matrix_request)).unwrap();
    let source_config = source_before_creation
        .hardware
        .as_mut()
        .unwrap()
        .boards
        .iter_mut()
        .find(|board| board.board_id == "left")
        .unwrap();
    source_config
        .assignments
        .insert("matrix/left-thumbs/r0c0/row".into(), "P1".into());
    source_config
        .assignments
        .insert("unrelated/function".into(), "P1".into());
    source_config
        .locks
        .insert("matrix/left-thumbs/r0c1/column".into(), "P2".into());
    source_config
        .locks
        .insert("unrelated/lock".into(), "P2".into());
    let expected = create_from(
        source_before_creation,
        json!({"at": {"x": 200, "y": 0}, "rotation": 0}),
        "literal-mirror",
        None,
    );

    let document = mirror_added_matrix(&next, &addition).unwrap().draft;
    let mirror = document
        .hardware
        .as_ref()
        .unwrap()
        .boards
        .iter()
        .find(|board| board.board_id == "right")
        .unwrap();
    assert_eq!(
        mirror
            .assignments
            .get("matrix/right-thumbs/r0c0/row")
            .map(String::as_str),
        Some("P1")
    );
    assert_eq!(
        mirror
            .locks
            .get("matrix/right-thumbs/r0c1/column")
            .map(String::as_str),
        Some("P2")
    );
    assert_eq!(
        mirror.assignments.get("unrelated/function"),
        mirror_before.assignments.get("unrelated/function")
    );
    assert_eq!(
        mirror.locks.get("unrelated/lock"),
        mirror_before.locks.get("unrelated/lock")
    );
    assert!(!mirror.assignments.contains_key("unrelated/function"));
    assert!(!mirror.locks.contains_key("unrelated/lock"));
    let matrix_wiring = |values: &BTreeMap<String, String>| {
        values
            .iter()
            .filter(|(function, _)| function.starts_with("matrix/right-thumbs/"))
            .map(|(function, pin)| (function.clone(), pin.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    let expected_mirror = expected
        .hardware
        .as_ref()
        .unwrap()
        .boards
        .iter()
        .find(|board| board.board_id == "right")
        .unwrap();
    assert_eq!(
        matrix_wiring(&mirror.assignments),
        matrix_wiring(&expected_mirror.assignments),
        "added assignments match CreateMirroredHalf"
    );
    assert_eq!(
        matrix_wiring(&mirror.locks),
        matrix_wiring(&expected_mirror.locks),
        "added locks match CreateMirroredHalf"
    );
}

#[test]
fn local_electrical_board_keeps_its_setup_and_omitted_gets_none() {
    for state in ["local", "omitted"] {
        let (addition, mut next) = plan(
            &pair_document(),
            &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
        );
        let source = next
            .hardware
            .as_mut()
            .unwrap()
            .boards
            .iter_mut()
            .find(|board| board.board_id == "left")
            .unwrap();
        source
            .assignments
            .insert("matrix/left-thumbs/r0c0/row".into(), "P1".into());
        source
            .locks
            .insert("matrix/left-thumbs/r0c0/row".into(), "P1".into());
        let mirror_before = if state == "local" {
            let mirror = next
                .hardware
                .as_mut()
                .unwrap()
                .boards
                .iter_mut()
                .find(|board| board.board_id == "right")
                .unwrap();
            mirror
                .assignments
                .insert("local/custom".into(), "P2".into());
            mirror.locks.insert("local/custom".into(), "P2".into());
            Some(mirror.clone())
        } else {
            next.hardware
                .as_mut()
                .unwrap()
                .boards
                .retain(|board| board.board_id != "right");
            None
        };
        next.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .exceptions
            .push(
                serde_json::from_value(
                    json!({"kind": "electrical-board", "mirrorId": "right", "state": state}),
                )
                .unwrap(),
            );

        let document = mirror_added_matrix(&next, &addition).unwrap().draft;
        let mirror = document
            .hardware
            .as_ref()
            .unwrap()
            .boards
            .iter()
            .find(|board| board.board_id == "right");
        match (state, mirror, mirror_before) {
            ("local", Some(mirror), Some(before)) => {
                assert_eq!(
                    mirror, &before,
                    "{state} electrical configuration is preserved"
                );
            }
            ("omitted", None, None) => {}
            _ => panic!("unexpected electrical-board state for {state}"),
        }
    }
}

#[test]
fn a_new_matrix_assignment_without_a_mirror_controller_pin_is_refused_by_identity() {
    for lock in [false, true] {
        let (addition, mut next) = plan(
            &pair_document(),
            &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
        );
        let source = next
            .hardware
            .as_mut()
            .unwrap()
            .boards
            .iter_mut()
            .find(|board| board.board_id == "left")
            .unwrap();
        if lock {
            source
                .locks
                .insert("matrix/left-thumbs/r0c0/row".into(), "MISSING".into());
        } else {
            source
                .assignments
                .insert("matrix/left-thumbs/r0c0/row".into(), "MISSING".into());
        }
        let message = refusal(&next, &addition);
        assert!(message.contains("matrix/left-thumbs/r0c0/row"), "{message}");
        assert!(message.contains("MISSING"), "{message}");
    }
}

#[test]
fn a_matrix_also_linked_by_a_legacy_layout_mirror_link_is_refused() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    next.layouts = serde_json::from_value(json!([
        {"id": "legacy-a", "name": "A", "boardId": "left", "matrixIds": ["left-thumbs"],
         "partIds": [], "mirrorLink": {"sourceId": "legacy-b", "axisX": 0}},
        {"id": "legacy-b", "name": "B", "boardId": "left", "matrixIds": ["left-keys"], "partIds": []}
    ]))
    .unwrap();
    let message = refusal(&next, &addition);
    assert!(message.contains("legacy"), "{message}");
}

#[test]
fn a_plan_for_boards_no_pair_relates_is_refused_and_the_document_is_untouched() {
    let (mut addition, next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    let before = serde_json::to_value(&next).unwrap();
    addition.mirror_board_id = "elsewhere".into();
    assert!(refusal(&next, &addition).contains("elsewhere"));
    assert_eq!(serde_json::to_value(&next).unwrap(), before);
}

#[test]
fn a_refused_transition_changes_nothing_and_a_success_does_not_edit_its_input() {
    let (addition, next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    let before = serde_json::to_value(&next).unwrap();
    let output = mirror_added_matrix(&next, &addition).unwrap();
    assert_eq!(serde_json::to_value(&next).unwrap(), before);
    assert!(output.changed_ids.contains(&"right-thumbs".to_owned()));
    assert!(
        output
            .changed_ids
            .contains(&"matrix/right-thumbs/r0c0".to_owned())
    );
}

#[test]
fn a_layout_of_the_new_matrix_is_mirrored_with_its_independent_parts() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    next.layouts = serde_json::from_value(json!([
        {"id": "left-thumbs-layout", "name": "Thumbs", "boardId": "left",
         "matrixIds": ["left-thumbs"], "partIds": []}
    ]))
    .unwrap();
    let document = mirror_added_matrix(&next, &addition).unwrap().draft;
    let mirror = correspondence(&document, MirroredItemKind::Layout, "left-thumbs-layout")
        .expect("the layout is linked");
    let layout = document.layouts.iter().find(|l| l.id == mirror).unwrap();
    assert_eq!(layout.board_id, "right");
    assert_eq!(layout.matrix_ids, ["right-thumbs"]);
    assert_eq!(layout.name, "Thumbs");
    assert!(layout.mirror_link.is_none());
}

#[test]
fn a_net_that_a_new_member_pin_uses_is_mirrored_onto_the_mirror_board() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    next.nets.push(crate::model::Net {
        id: "left-thumb-row".into(),
        name: "LEFT_THUMB_ROW".into(),
        pins: vec![
            crate::model::Pin {
                part_id: "matrix/left-thumbs/r0c0".into(),
                pad_id: "pad-1".into(),
            },
            crate::model::Pin {
                part_id: "left/U1".into(),
                pad_id: "P1".into(),
            },
        ],
    });
    next.boards
        .iter_mut()
        .find(|board| board.id == "left")
        .unwrap()
        .net_ids
        .push("left-thumb-row".into());
    let document = mirror_added_matrix(&next, &addition).unwrap().draft;
    let mirror = correspondence(&document, MirroredItemKind::Net, "left-thumb-row")
        .expect("the net is linked");
    let net = document.nets.iter().find(|n| n.id == mirror).unwrap();
    assert_eq!(net.name, "RIGHT_THUMB_ROW");
    let pins: Vec<_> = net.pins.iter().map(|p| p.part_id.as_str()).collect();
    assert_eq!(pins, ["matrix/right-thumbs/r0c0", "right/U1"]);
    assert!(
        document
            .boards
            .iter()
            .find(|board| board.id == "right")
            .unwrap()
            .net_ids
            .contains(&mirror)
    );
}

#[test]
fn a_net_the_mirror_already_has_receives_the_new_pins_without_a_second_net() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    for (id, board, part) in [
        ("left-bus", "left", "left/U1"),
        ("right-bus", "right", "right/U1"),
    ] {
        next.nets.push(crate::model::Net {
            id: id.into(),
            name: id.to_uppercase(),
            pins: vec![crate::model::Pin {
                part_id: part.into(),
                pad_id: "P2".into(),
            }],
        });
        next.boards
            .iter_mut()
            .find(|b| b.id == board)
            .unwrap()
            .net_ids
            .push(id.into());
    }
    let halves = next.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(
        serde_json::from_value(
            json!({"kind": "net", "sourceId": "left-bus", "mirrorId": "right-bus"}),
        )
        .unwrap(),
    );
    next.nets
        .iter_mut()
        .find(|net| net.id == "left-bus")
        .unwrap()
        .pins
        .push(crate::model::Pin {
            part_id: "matrix/left-thumbs/r0c1".into(),
            pad_id: "pad-2".into(),
        });
    let nets_before = next.nets.len();
    let document = mirror_added_matrix(&next, &addition).unwrap().draft;
    assert_eq!(document.nets.len(), nets_before);
    let mirror = document.nets.iter().find(|n| n.id == "right-bus").unwrap();
    assert!(
        mirror
            .pins
            .iter()
            .any(|pin| pin.part_id == "matrix/right-thumbs/r0c1" && pin.pad_id == "pad-2")
    );
}

#[test]
fn a_module_connected_to_a_new_member_is_mirrored_with_its_correspondence() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    let module = serde_json::from_value(json!({
        "id": "left-ring", "definitionId": "module/ring", "hostBoardId": "left", "hostInstanceId": "left",
        "hostFace": "front", "facingFace": "back", "at": {"x": 40, "y": 130}, "rotation": 30,
        "gap": 1.0, "attachment": "board",
        "connection": {"hostConnectorPartId": "matrix/left-thumbs/r0c0", "modulePortId": "p",
            "busId": "left-bus", "assignments": {}, "cableType": "ffc"}
    }))
    .expect("a module record");
    next.modules.push(module);
    let document = mirror_added_matrix(&next, &addition).unwrap().draft;
    let mirror = correspondence(&document, MirroredItemKind::Module, "left-ring")
        .expect("the module is linked");
    let placed = document.modules.iter().find(|m| m.id == mirror).unwrap();
    assert_eq!(placed.host_board_id, "right");
    assert_eq!(
        placed.connection.as_ref().unwrap().host_connector_part_id,
        "matrix/right-thumbs/r0c0"
    );
    assert_eq!(placed.at.x, 160.0);
    assert_eq!(placed.host_instance_id.as_deref(), Some("right"));
}

#[test]
fn keycap_settings_of_the_new_matrix_and_keys_travel_to_the_mirror() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    next.keycaps = Some(
        serde_json::from_value(json!({
            "boards": {}, "matrices": {"left-thumbs": {"profile": "cherry"}},
            "keys": {"matrix/left-thumbs/r0c0": {"legend": "T"}}
        }))
        .expect("keycap settings"),
    );
    let document = mirror_added_matrix(&next, &addition).unwrap().draft;
    let keycaps = document.keycaps.unwrap();
    assert!(keycaps.matrices.contains_key("right-thumbs"));
    assert!(keycaps.keys.contains_key("matrix/right-thumbs/r0c0"));
}

/// The strongest oracle available: the mirror this transition adds to an existing pair is the
/// mirror the production creation path derives for the same source matrix, under every
/// companion mode and a translated, a slanted and a rotated map.
#[test]
fn the_added_mirror_is_the_mirror_creation_derives_for_the_same_source() {
    let maps = [(200.0, 0.0, 0.0), (310.0, 42.0, 27.0), (150.0, -35.0, 90.0)];
    let tweak = json!({"x": 0.75, "y": -0.5, "rotation": 8.0});
    let modes = [
        ("literal-mirror", None),
        ("duplicate-position", None),
        ("literal-mirror-with-tweak", Some(tweak.clone())),
    ];
    // Every map and every companion mode still runs; the redundant cross combinations do not.
    for (map, mode) in [(0, 0), (0, 1), (0, 2), (1, 0), (2, 0)] {
        let (x, y, rotation) = maps[map];
        let (mode, tweak) = modes[mode].clone();
        let mapping = json!({"at": {"x": x, "y": y}, "rotation": rotation});
        let label = format!("map ({x}, {y}, {rotation}) {mode}");
        let mut request = new_matrix("left-thumbs", "left", (10.0, 120.0), 3);
        request["matrix"]["rotation"] = json!(12);
        request["matrix"]["columnLayout"] = json!([{}, {"splay": 6.0}, {"splay": -3.0}]);
        request["matrix"]["cells"][0]["offset"] = json!({"x": 0.5, "y": -0.25});
        request["matrix"]["cells"][0]["rotation"] = json!(4);
        request["matrix"]["cells"][0]["assemblies"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": "stab", "definitionId": STABILISER,
                "offset": {"x": 0, "y": -12}, "rotation": 30, "side": "front"}));

        // Reference: the matrix exists before the half is created.
        let mut before = source_half();
        crate::apply(&mut before, &[], &op(request.clone())).unwrap();
        let reference = create_from(before, mapping.clone(), mode, tweak.clone());
        // Under test: the matrix is added to a pair that already exists.
        let pair = create_from(source_half(), mapping.clone(), mode, tweak.clone());
        let added = add_matrix(&pair, &request);

        assert_eq!(
            serde_json::to_value(matrix(&added, "right-thumbs")).unwrap(),
            serde_json::to_value(matrix(&reference, "right-thumbs")).unwrap(),
            "{label}: matrix"
        );
        let comparable = |document: &ProjectDoc, id: &str| {
            let mut part =
                serde_json::to_value(document.parts.iter().find(|part| part.id == id).unwrap())
                    .unwrap();
            part.as_object_mut().unwrap().remove("reference");
            part
        };
        let members = &matrix(&reference, "right-thumbs").part_ids;
        assert_eq!(
            members.len(),
            3 + 2,
            "{label}: three keys, one diode, one stabiliser"
        );
        for id in members {
            assert_eq!(
                comparable(&added, id),
                comparable(&reference, id),
                "{label}: {id}"
            );
        }
        let links = |document: &ProjectDoc| {
            let mut links: Vec<_> = halves(document)
                .correspondences
                .iter()
                .filter(|item| item.source_id.contains("left-thumbs"))
                .map(|item| serde_json::to_value(item).unwrap())
                .collect();
            links.sort_by_key(|value| value.to_string());
            links
        };
        assert_eq!(links(&added), links(&reference), "{label}: correspondences");
    }
}

#[test]
fn a_net_of_an_embedded_circuit_that_a_new_member_joins_is_refused_by_name() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    next.nets.push(crate::model::Net {
        id: "embedded/left-haptic/drive".into(),
        name: "DRIVE".into(),
        pins: vec![crate::model::Pin {
            part_id: "matrix/left-thumbs/r0c0".into(),
            pad_id: "pad-1".into(),
        }],
    });
    let message = refusal(&next, &addition);
    assert!(message.contains("embedded/left-haptic/drive"), "{message}");
    assert!(message.contains("112"), "{message}");
}

#[test]
fn a_tweak_pair_with_no_stored_tweak_has_none_to_give_a_new_companion() {
    let pair = create_from(
        source_half(),
        json!({"at": {"x": 200, "y": 0}, "rotation": 0}),
        "literal-mirror-with-tweak",
        None,
    );
    let (addition, next) = plan(&pair, &new_matrix("left-thumbs", "left", (10.0, 120.0), 2));
    let message = refusal(&next, &addition);
    assert!(message.contains("tweak"), "{message}");
}

#[test]
fn a_reused_board_has_no_second_board_to_receive_the_matrix() {
    let (mut addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    next.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .mirror_board_id = "left".into();
    addition.mirror_board_id = "left".into();
    let message = refusal(&next, &addition);
    assert!(message.contains("reuses one board"), "{message}");
}

#[test]
fn a_layout_part_with_no_linked_mirror_refuses_rather_than_dropping_it() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    next.parts.push({
        let mut part = next.parts[0].clone();
        part.id = "left/EXTRA".into();
        part.reference = "X1".into();
        part
    });
    next.boards
        .iter_mut()
        .find(|board| board.id == "left")
        .unwrap()
        .part_ids
        .push("left/EXTRA".into());
    next.layouts = serde_json::from_value(json!([
        {"id": "thumbs-layout", "name": "Thumbs", "boardId": "left",
         "matrixIds": ["left-thumbs"], "partIds": ["left/EXTRA"]}
    ]))
    .unwrap();
    let message = refusal(&next, &addition);
    assert!(message.contains("left/EXTRA"), "{message}");
}

#[test]
fn member_attributes_the_creation_path_copies_travel_to_the_mirror() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    let key = next
        .parts
        .iter_mut()
        .find(|part| part.id == "matrix/left-thumbs/r0c0")
        .unwrap();
    key.keycap = Some(crate::model::Vec2 { x: 1.5, y: 1.0 });
    key.locked = Some(true);
    key.properties = Some(std::collections::BTreeMap::from([(
        "note".to_owned(),
        json!("keep"),
    )]));
    let document = mirror_added_matrix(&next, &addition).unwrap().draft;
    let mirror = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/right-thumbs/r0c0")
        .unwrap();
    assert_eq!(mirror.keycap, Some(crate::model::Vec2 { x: 1.5, y: 1.0 }));
    assert_eq!(mirror.locked, Some(true));
    assert_eq!(mirror.properties.as_ref().unwrap()["note"], json!("keep"));
}

#[test]
fn a_local_mirror_net_is_left_alone_when_a_new_member_joins_its_source() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    for (id, board, part) in [
        ("left-bus", "left", "left/U1"),
        ("right-bus", "right", "right/U1"),
    ] {
        next.nets.push(crate::model::Net {
            id: id.into(),
            name: id.to_uppercase(),
            pins: vec![crate::model::Pin {
                part_id: part.into(),
                pad_id: "P2".into(),
            }],
        });
        next.boards
            .iter_mut()
            .find(|b| b.id == board)
            .unwrap()
            .net_ids
            .push(id.into());
    }
    let halves = next.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(
        serde_json::from_value(
            json!({"kind": "net", "sourceId": "left-bus", "mirrorId": "right-bus"}),
        )
        .unwrap(),
    );
    halves.exceptions.push(
        serde_json::from_value(json!({"kind": "net", "mirrorId": "right-bus", "state": "local"}))
            .unwrap(),
    );
    next.nets
        .iter_mut()
        .find(|net| net.id == "left-bus")
        .unwrap()
        .pins
        .push(crate::model::Pin {
            part_id: "matrix/left-thumbs/r0c1".into(),
            pad_id: "pad-2".into(),
        });
    let document = mirror_added_matrix(&next, &addition).unwrap().draft;
    let mirror = document.nets.iter().find(|n| n.id == "right-bus").unwrap();
    assert_eq!(mirror.pins.len(), 1, "the Local net keeps its own pins");
}

#[test]
fn a_source_matrix_that_is_not_on_the_source_board_is_refused() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    next.matrices
        .iter_mut()
        .find(|matrix| matrix.id == "left-thumbs")
        .unwrap()
        .board_id = Some("right".into());
    let message = refusal(&next, &addition);
    assert!(message.contains("The source matrix"), "{message}");
    assert!(message.contains("left-thumbs"), "{message}");
}

#[test]
fn companions_kept_local_to_the_source_are_not_mirrored_or_linked() {
    let mut request = new_matrix("left-thumbs", "left", (10.0, 120.0), 2);
    request["matrix"]["cells"][0]["assembliesLocal"] = json!(true);
    let document = add_matrix(&pair_document(), &request);
    assert!(
        document
            .parts
            .iter()
            .any(|part| part.id == "matrix/left-thumbs/r0c0/diode")
    );
    assert!(
        !document
            .parts
            .iter()
            .any(|part| part.id == "matrix/right-thumbs/r0c0/diode")
    );
    assert_eq!(
        correspondence(
            &document,
            MirroredItemKind::Part,
            "matrix/left-thumbs/r0c0/diode"
        ),
        None
    );
    assert_eq!(
        correspondence(&document, MirroredItemKind::Part, "matrix/left-thumbs/r0c0").as_deref(),
        Some("matrix/right-thumbs/r0c0")
    );
}

#[test]
fn a_draft_that_does_not_validate_as_a_mirrored_record_is_refused_whole() {
    let (addition, mut next) = plan(
        &pair_document(),
        &new_matrix("left-thumbs", "left", (10.0, 120.0), 2),
    );
    // A legacy exception naming a mirror item that is not on the mirror board.
    next.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(
            serde_json::from_value(
                json!({"kind": "part", "mirrorId": "right/ghost", "state": "local"}),
            )
            .unwrap(),
        );
    let message = refusal(&next, &addition);
    assert!(message.contains("right/ghost"), "{message}");
}
