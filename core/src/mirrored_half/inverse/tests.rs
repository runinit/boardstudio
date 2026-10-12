//! Direct translation tests for the linked-mirror inverse seam (ticket 110).
//!
//! Every expectation is a literal worked from the pair map by hand: the axis map sends
//! `(x, y)` to `(200 - x, y)`, and the rotated map `(100, -30, 90 degrees)` sends the source
//! point `(a, b)` to `(100 - b, -30 - a)`. No expectation calls the production mapping.

use super::*;
use crate::model::{EditOperation, Pose2, Vec2};
use serde_json::{Value, json};

const AXIS: Pose2 = Pose2 {
    at: Vec2 { x: 200.0, y: 0.0 },
    rotation: 0.0,
};
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

fn op(value: Value) -> EditOperation {
    serde_json::from_value(value).expect("the operation is valid")
}

/// Compares two JSON trees, numbers within a tolerance (rotated maps carry cosine noise).
fn assert_close(actual: &Value, expected: &Value, path: &str) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            assert!((a - e).abs() < 1e-9, "{path}: {a} is not {e}");
        }
        (Value::Object(a), Value::Object(e)) => {
            assert_eq!(
                a.keys().collect::<Vec<_>>(),
                e.keys().collect::<Vec<_>>(),
                "{path}: keys"
            );
            for (key, value) in e {
                assert_close(&a[key], value, &format!("{path}.{key}"));
            }
        }
        (Value::Array(a), Value::Array(e)) => {
            assert_eq!(
                a.len(),
                e.len(),
                "{path}: length of {actual} against {expected}"
            );
            for (index, (a, e)) in a.iter().zip(e).enumerate() {
                assert_close(a, e, &format!("{path}[{index}]"));
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}

fn canonical(document: &ProjectDoc, operation: &Value) -> Result<Canonical, String> {
    canonicalize(document, &op(operation.clone()))
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

#[test]
fn coverage_names_this_slice_and_the_owner_of_every_other_family() {
    let owned = |value: Value| coverage(&op(value));
    let translated = [
        json!({"kind": "set-matrix", "matrix": {"id": "m", "rows": 1, "columns": 1,
            "pitch": {"x": 1, "y": 1}, "origin": {"x": 0, "y": 0}, "definitionId": "d", "partIds": []}}),
        json!({"kind": "set-matrix-splay", "matrixId": "m", "column": 0,
            "change": {"kind": "origin", "world": null}}),
        json!({"kind": "set-matrix-placement", "matrixId": "m",
            "change": {"kind": "column-stagger", "column": 0, "value": 1}}),
        json!({"kind": "remove-matrix", "id": "m"}),
        json!({"kind": "set-matrix-cells-enabled", "cells": {}, "enabled": true}),
        json!({"kind": "remove-selected-matrix-cells", "cells": {}}),
        json!({"kind": "remove-matrix-cells", "matrixId": "m", "cells": []}),
        json!({"kind": "remove-matrix-row", "matrixId": "m", "row": 0}),
        json!({"kind": "remove-matrix-column", "matrixId": "m", "column": 0}),
        json!({"kind": "move-parts", "positions": []}),
        json!({"kind": "remove-parts", "ids": []}),
    ];
    for request in translated {
        assert_eq!(owned(request.clone()), Coverage::Translated, "{request}");
    }
    assert_eq!(
        owned(json!({"kind": "remove-constraint", "id": "c"})),
        Coverage::OwnedBy("114")
    );
    assert_eq!(
        owned(json!({"kind": "remove-mounted-module", "id": "m"})),
        Coverage::OwnedBy("112")
    );
    assert_eq!(
        owned(json!({"kind": "select-outline", "boardId": "b"})),
        Coverage::OwnedBy("113")
    );
}

// ---- unrelated and local targets -----------------------------------------------------------

#[test]
fn an_unpaired_document_and_source_side_requests_are_returned_unchanged() {
    let request = json!({"kind": "move-parts", "positions": [
        {"id": "matrix/left-keys/r0c0", "at": {"x": 3, "y": 4}},
        {"id": "left/U1", "at": {"x": -20, "y": 1}},
        {"id": "right/FREE", "at": {"x": 310, "y": 5}}]});
    // The unpaired mirror-board part and source parts execute as written.
    expect_operation(&fixture(), request.clone(), request.clone());
    let mut unpaired = fixture();
    unpaired.hardware.as_mut().unwrap().pairs[0].halves = None;
    let moved = json!({"kind": "move-parts", "positions": [
        {"id": "right/U1", "at": {"x": 1, "y": 2}}]});
    expect_operation(&unpaired, moved.clone(), moved);
    let result = canonical(&fixture(), &request).unwrap();
    assert!(!result.translated);
    assert!(result.additions.is_empty());
}

// ---- MoveParts -----------------------------------------------------------------------------

#[test]
fn moving_a_linked_independent_part_moves_its_source_through_the_inverse_pair_map() {
    expect_operation(
        &fixture(),
        json!({"kind": "move-parts", "positions": [{"id": "right/U1", "at": {"x": 230, "y": 7}}]}),
        json!({"kind": "move-parts", "positions": [{"id": "left/U1", "at": {"x": -30, "y": 7}}]}),
    );
    // Rotated map: mirror (90, -50) is the image of source (20, 10).
    expect_operation(
        &rotated_fixture(),
        json!({"kind": "move-parts", "positions": [{"id": "right/U1", "at": {"x": 90, "y": -50}}]}),
        json!({"kind": "move-parts", "positions": [{"id": "left/U1", "at": {"x": 20, "y": 10}}]}),
    );
}

#[test]
fn moving_a_linked_key_switch_moves_the_source_key() {
    expect_operation(
        &fixture(),
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/right-keys/r0c1", "at": {"x": 175, "y": 6}}]}),
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/left-keys/r0c1", "at": {"x": 25, "y": 6}}]}),
    );
}

#[test]
fn a_mixed_batch_translates_only_linked_mirror_targets_and_keeps_order() {
    expect_operation(
        &fixture(),
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/left-keys/r0c0", "at": {"x": 1, "y": 1}},
            {"id": "right/U1", "at": {"x": 230, "y": 7}},
            {"id": "right/HEADER", "at": {"x": 220, "y": 31}},
            {"id": "right/FREE", "at": {"x": 301, "y": 2}},
            {"id": "matrix/right-keys/r0c1", "at": {"x": 175, "y": 6}}]}),
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/left-keys/r0c0", "at": {"x": 1, "y": 1}},
            {"id": "left/U1", "at": {"x": -30, "y": 7}},
            {"id": "right/HEADER", "at": {"x": 220, "y": 31}},
            {"id": "right/FREE", "at": {"x": 301, "y": 2}},
            {"id": "matrix/left-keys/r0c1", "at": {"x": 25, "y": 6}}]}),
    );
}

#[test]
fn naming_both_halves_of_one_link_is_one_move_when_they_agree_and_a_refusal_when_they_do_not() {
    // Source (-30, 7) is the mirror's (230, 7): one canonical value, so one entry.
    expect_operation(
        &fixture(),
        json!({"kind": "move-parts", "positions": [
            {"id": "left/U1", "at": {"x": -30, "y": 7}},
            {"id": "right/U1", "at": {"x": 230, "y": 7}}]}),
        json!({"kind": "move-parts", "positions": [
            {"id": "left/U1", "at": {"x": -30, "y": 7}}]}),
    );
    let message = refusal(
        &fixture(),
        json!({"kind": "move-parts", "positions": [
            {"id": "left/U1", "at": {"x": -20, "y": 0}},
            {"id": "right/U1", "at": {"x": 230, "y": 7}}]}),
    );
    assert!(
        message.contains("left/U1") && message.contains("right/U1"),
        "{message}"
    );
}

#[test]
fn an_omitted_mirror_target_is_refused_not_recreated() {
    let message = refusal(
        &fixture(),
        json!({"kind": "move-parts", "positions": [{"id": "right/GONE", "at": {"x": 1, "y": 1}}]}),
    );
    assert!(
        message.contains("right/GONE") && message.contains("omitted"),
        "{message}"
    );
}

// ---- AddPart -------------------------------------------------------------------------------

#[test]
fn a_part_added_on_the_mirror_board_becomes_a_source_addition_with_its_mirror_planned() {
    let request = json!({"kind": "add-part", "boardId": "right", "part": {
        "id": "right/LED", "definitionId": "u", "reference": "right-D9",
        "pose": {"at": {"x": 230, "y": 12}, "rotation": 180}, "side": "front",
        "properties": {"note": "kept"}}});
    let result = canonical(&fixture(), &request).unwrap();
    assert_close(
        &serde_json::to_value(&result.operation).unwrap(),
        &json!({"kind": "add-part", "boardId": "left", "part": {
            "id": "left/LED", "definitionId": "u", "reference": "left-D9",
            "pose": {"at": {"x": -30, "y": 12}, "rotation": 0}, "side": "front",
            "properties": {"note": "kept"}}}),
        "operation",
    );
    assert!(result.translated);
    assert_eq!(
        result.additions,
        vec![PartAddition {
            source_id: "left/LED".into(),
            mirror_id: "right/LED".into(),
            mirror_definition_id: "u".into(),
            mirror_reference: "right-D9".into(),
        }]
    );
}

#[test]
fn a_source_side_addition_is_unchanged() {
    let source = json!({"kind": "add-part", "boardId": "left", "part": {
        "id": "left/LED", "definitionId": "u", "reference": "D9",
        "pose": {"at": {"x": 1, "y": 2}, "rotation": 30}, "side": "front"}});
    expect_operation(&fixture(), source.clone(), source);
}

#[test]
fn a_mirror_side_addition_whose_source_identity_is_taken_allocates_the_next_free_one() {
    let request = json!({"kind": "add-part", "boardId": "right", "part": {
        "id": "right/U1-new", "definitionId": "u", "reference": "RU9",
        "pose": {"at": {"x": 210, "y": 1}, "rotation": 0}, "side": "front"}});
    let mut taken = fixture();
    taken
        .parts
        .push(serde_json::from_value(part("left/U1-new", "u", "X1", 0.0, 0.0, 0.0)).unwrap());
    let result = canonical(&taken, &request).unwrap();
    let EditOperation::AddPart { part, board_id } = result.operation else {
        panic!("an addition is expected");
    };
    assert_eq!(board_id.as_deref(), Some("left"));
    assert_eq!(part.id, "left/U1-new-2");
    assert_eq!(result.additions[0].source_id, "left/U1-new-2");
    assert_eq!(result.additions[0].mirror_id, "right/U1-new");
}

// ---- RemoveParts ---------------------------------------------------------------------------

#[test]
fn removing_linked_mirror_parts_removes_their_sources_once() {
    expect_operation(
        &fixture(),
        json!({"kind": "remove-parts", "ids": [
            "right/U1", "right/FREE", "left/U1", "right/HEADER",
            "matrix/right-keys/r0c0/diode"]}),
        json!({"kind": "remove-parts", "ids": [
            "left/U1", "right/FREE", "right/HEADER", "matrix/left-keys/r0c0/diode"]}),
    );
}

#[test]
fn removing_an_omitted_mirror_part_is_refused() {
    let message = refusal(
        &fixture(),
        json!({"kind": "remove-parts", "ids": ["right/GONE"]}),
    );
    assert!(
        message.contains("right/GONE") && message.contains("omitted"),
        "{message}"
    );
}

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

#[test]
fn a_literal_mirror_companion_moves_its_source_to_the_mirrored_point() {
    expect_operation(
        &fixture(),
        move_diode((190.0, -5.0)),
        moved_source_diode((10.0, -5.0)),
    );
}

#[test]
fn a_tweaked_companion_removes_the_tweak_turned_by_the_mirror_key_heading() {
    let mut document = fixture();
    set_companion(
        &mut document,
        "literal-mirror-with-tweak",
        json!({"x": 1.0, "y": 2.0, "rotation": 0.0}),
    );
    // Mirror key heading 0: the tweak (1, 2) is removed as it stands.
    expect_operation(
        &document,
        move_diode((190.0, -5.0)),
        moved_source_diode((11.0, -7.0)),
    );
    // Mirror key heading 90: the tweak is (-2, 1) in the world.
    pose_of(&mut document, "matrix/right-keys/r0c0").rotation = 90.0;
    expect_operation(
        &document,
        move_diode((190.0, -5.0)),
        moved_source_diode((8.0, -6.0)),
    );
}

#[test]
fn a_duplicate_position_companion_keeps_the_mirror_offset_in_the_source_key_frame() {
    let mut document = fixture();
    set_companion(&mut document, "duplicate-position", json!(null));
    // Offset from the mirror key (200, 0) is (-10, -5); the source key (0, 0) heads 0.
    expect_operation(
        &document,
        move_diode((190.0, -5.0)),
        moved_source_diode((-10.0, -5.0)),
    );
    // A source key heading 90 turns that offset to (5, -10).
    pose_of(&mut document, "matrix/left-keys/r0c0").rotation = 90.0;
    expect_operation(
        &document,
        move_diode((190.0, -5.0)),
        moved_source_diode((5.0, -10.0)),
    );
}

#[test]
fn a_duplicate_position_companion_follows_its_key_when_both_move_in_one_request() {
    let mut document = fixture();
    set_companion(&mut document, "duplicate-position", json!(null));
    // The mirror key goes to (210, 0): its source (-10, 0); the offset is (-20, -5).
    expect_operation(
        &document,
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/right-keys/r0c0", "at": {"x": 210, "y": 0}},
            {"id": DIODE, "at": {"x": 190, "y": -5}}]}),
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/left-keys/r0c0", "at": {"x": -10, "y": 0}},
            {"id": "matrix/left-keys/r0c0/diode", "at": {"x": -30, "y": -5}}]}),
    );
}

#[test]
fn a_stabiliser_keeps_the_literal_position_in_every_mode() {
    let mut document = fixture();
    set_companion(&mut document, "duplicate-position", json!(null));
    document
        .parts
        .iter_mut()
        .find(|part| part.id == DIODE)
        .unwrap()
        .definition_id = "kicad:marbastlib/STAB_MX_2u".into();
    expect_operation(
        &document,
        move_diode((190.0, -5.0)),
        moved_source_diode((10.0, -5.0)),
    );
}

// ---- matrix parameters and structure ---------------------------------------------------------

fn with_legacy_link(mut document: ProjectDoc) -> ProjectDoc {
    document.layouts = serde_json::from_value(json!([
        {"id": "legacy-a", "name": "A", "boardId": "left", "matrixId": "left-keys",
         "partIds": [], "mirrorLink": {"sourceId": "legacy-b", "axisX": 0}},
        {"id": "legacy-b", "name": "B", "boardId": "left", "matrixId": "right-keys", "partIds": []}
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

#[test]
fn matrix_structure_requests_on_the_mirror_name_the_source_matrix() {
    for (request, expected) in [
        (
            json!({"kind": "remove-matrix", "id": "right-keys"}),
            json!({"kind": "remove-matrix", "id": "left-keys"}),
        ),
        (
            json!({"kind": "remove-matrix-cells", "matrixId": "right-keys", "cells": [[0, 1]]}),
            json!({"kind": "remove-matrix-cells", "matrixId": "left-keys", "cells": [[0, 1]]}),
        ),
        (
            json!({"kind": "remove-matrix-row", "matrixId": "right-keys", "row": 0}),
            json!({"kind": "remove-matrix-row", "matrixId": "left-keys", "row": 0}),
        ),
        (
            json!({"kind": "remove-matrix-column", "matrixId": "right-keys", "column": 1}),
            json!({"kind": "remove-matrix-column", "matrixId": "left-keys", "column": 1}),
        ),
    ] {
        expect_operation(&fixture(), request, expected);
    }
}

#[test]
fn selected_cell_requests_move_to_the_source_key_and_union_both_halves() {
    for kind in ["set-matrix-cells-enabled", "remove-selected-matrix-cells"] {
        let build = |cells: Value| {
            let mut request = json!({"kind": kind, "cells": cells});
            if kind == "set-matrix-cells-enabled" {
                request["enabled"] = json!(false);
            }
            request
        };
        // The mirror's list is kept as written when nothing else names the link.
        expect_operation(
            &fixture(),
            build(json!({"right-keys": [[0, 1], [0, 0]], "elsewhere": [[2, 2]]})),
            build(json!({"left-keys": [[0, 1], [0, 0]], "elsewhere": [[2, 2]]})),
        );
        // Both halves of one link are one set of coordinates, ordered and without repeats.
        expect_operation(
            &fixture(),
            build(json!({"right-keys": [[0, 1]], "left-keys": [[0, 1], [0, 0]]})),
            build(json!({"left-keys": [[0, 0], [0, 1]]})),
        );
    }
}

#[test]
fn matrix_splay_origin_is_a_world_point_and_angles_are_matrix_frame_values() {
    expect_operation(
        &rotated_fixture(),
        json!({"kind": "set-matrix-splay", "matrixId": "right-keys", "column": 1,
            "change": {"kind": "origin", "world": {"x": 90, "y": -50}}}),
        json!({"kind": "set-matrix-splay", "matrixId": "left-keys", "column": 1,
            "change": {"kind": "origin", "world": {"x": 20, "y": 10}}}),
    );
    expect_operation(
        &fixture(),
        json!({"kind": "set-matrix-splay", "matrixId": "right-keys", "column": 1,
            "change": {"kind": "origin", "world": null}}),
        json!({"kind": "set-matrix-splay", "matrixId": "left-keys", "column": 1,
            "change": {"kind": "origin", "world": null}}),
    );
    expect_operation(
        &rotated_fixture(),
        json!({"kind": "set-matrix-splay", "matrixId": "right-keys", "column": 0,
            "change": {"kind": "angle", "angle": 7.5, "affect": "following"}}),
        json!({"kind": "set-matrix-splay", "matrixId": "left-keys", "column": 0,
            "change": {"kind": "angle", "angle": 7.5, "affect": "following"}}),
    );
}

#[test]
fn matrix_placement_values_are_matrix_frame_and_only_the_id_changes() {
    expect_operation(
        &rotated_fixture(),
        json!({"kind": "set-matrix-placement", "matrixId": "right-keys",
            "change": {"kind": "column-offset", "column": 1, "value": {"x": 2.5, "y": -1}}}),
        json!({"kind": "set-matrix-placement", "matrixId": "left-keys",
            "change": {"kind": "column-offset", "column": 1, "value": {"x": 2.5, "y": -1}}}),
    );
}

#[test]
fn source_and_local_matrix_requests_execute_as_written() {
    let request = json!({"kind": "remove-matrix-row", "matrixId": "left-keys", "row": 0});
    expect_operation(&fixture(), request.clone(), request);
    let local = json!({"kind": "remove-matrix", "id": "right-keys"});
    expect_operation(&with_matrix_state(fixture(), "local"), local.clone(), local);
}

#[test]
fn an_omitted_matrix_is_refused() {
    let message = refusal(
        &with_matrix_state(fixture(), "omitted"),
        json!({"kind": "remove-matrix", "id": "right-keys"}),
    );
    assert!(
        message.contains("right-keys") && message.contains("omitted"),
        "{message}"
    );
}

#[test]
fn a_matrix_linked_natively_and_by_a_legacy_layout_is_refused_from_either_half() {
    let document = with_legacy_link(fixture());
    for matrix in ["left-keys", "right-keys"] {
        let message = refusal(
            &document,
            json!({"kind": "remove-matrix-row", "matrixId": matrix, "row": 0}),
        );
        assert!(
            message.contains("legacy") && message.contains(matrix),
            "{message}"
        );
    }
}
