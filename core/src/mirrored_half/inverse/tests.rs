//! Direct translation tests for the linked-mirror inverse seam (ticket 110).
//!
//! Every expectation is a literal worked from the pair map by hand: the axis map sends
//! `(x, y)` to `(200 - x, y)`, and the rotated map `(100, -30, 90 degrees)` sends the source
//! point `(a, b)` to `(100 - b, -30 - a)`. No expectation calls the production mapping.

use super::*;
use crate::model::{Constraint, EditOperation, MirroredItemKind, Pose2, Vec2};
use serde_json::{Value, json};

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

#[test]
fn set_constraint_prepares_one_global_record_with_two_live_mirror_views() {
    let document = rotated_fixture();
    let constraint = Constraint::Offset {
        id: "constraint/durable-7".into(),
        source_part_id: "right/U1".into(),
        target_part_id: "matrix/right-keys/r0c0".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    };
    let canonical = canonicalize(
        &document,
        &EditOperation::SetConstraint {
            constraint: constraint.clone(),
        },
    )
    .unwrap();

    assert_eq!(
        canonical.operation,
        EditOperation::SetConstraint { constraint }
    );
    assert_eq!(canonical.prepared_constraints.len(), 1);
    let super::constraints::PreparedConstraintEdit::Upsert(
        crate::constraints::FramedConstraint::Framed {
            id,
            source,
            target,
            relation,
        },
    ) = &canonical.prepared_constraints[0]
    else {
        panic!("SetConstraint did not prepare one framed record");
    };
    assert_eq!(id, "constraint/durable-7");
    assert_eq!(
        source,
        &crate::constraints::ConstraintEndpoint::LinkedMirror {
            pair_id: "pair".into(),
            source_part_id: "left/U1".into(),
            mirror_part_id: "right/U1".into(),
        }
    );
    assert_eq!(
        target,
        &crate::constraints::ConstraintEndpoint::LinkedMirror {
            pair_id: "pair".into(),
            source_part_id: "matrix/left-keys/r0c0".into(),
            mirror_part_id: "matrix/right-keys/r0c0".into(),
        }
    );
    assert_eq!(
        relation,
        &crate::constraints::ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        }
    );
}

#[test]
fn a_new_framed_part_endpoint_naming_a_linked_mirror_normalizes_to_its_binding() {
    let document = rotated_fixture();
    let constraint = Constraint::Framed {
        id: "constraint/framed-command".into(),
        source: crate::model::ConstraintEndpoint::Part {
            part_id: "right/U1".into(),
        },
        target: crate::model::ConstraintEndpoint::Part {
            part_id: "left/U2".into(),
        },
        relation: crate::model::ConstraintRelation::Offset {
            offset: Vec2 { x: 3.0, y: -8.0 },
            rotation: 17.0,
        },
    };

    let canonical = canonicalize(&document, &EditOperation::SetConstraint { constraint })
        .expect("a new Part endpoint naming a linked mirror is normalized");

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &canonical.prepared_constraints[0]
    else {
        panic!("the Framed command compiles into one prepared record");
    };
    assert_eq!(
        record,
        &crate::constraints::FramedConstraint::Framed {
            id: "constraint/framed-command".into(),
            source: crate::constraints::ConstraintEndpoint::LinkedMirror {
                pair_id: "pair".into(),
                source_part_id: "left/U1".into(),
                mirror_part_id: "right/U1".into(),
            },
            target: crate::constraints::ConstraintEndpoint::Part {
                part_id: "left/U2".into(),
            },
            relation: crate::constraints::ConstraintRelation::Offset {
                offset: Vec2 { x: 3.0, y: -8.0 },
                rotation: 17.0,
            },
        }
    );
}

#[test]
fn local_constraint_edit_stays_local_and_remove_keeps_the_exact_global_id() {
    let document = rotated_fixture();
    let constraint = Constraint::Offset {
        id: "constraint/local-3".into(),
        source_part_id: "right/HEADER".into(),
        target_part_id: "right/HEADER".into(),
        offset: Vec2 { x: -4.0, y: 9.0 },
        rotation: -0.0,
    };
    let prepared = canonicalize(
        &document,
        &EditOperation::SetConstraint {
            constraint: constraint.clone(),
        },
    )
    .unwrap();
    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("the local SetConstraint must prepare its record");
    };
    assert_eq!(
        record,
        &crate::constraints::FramedConstraint::Framed {
            id: "constraint/local-3".into(),
            source: crate::constraints::ConstraintEndpoint::Part {
                part_id: "right/HEADER".into(),
            },
            target: crate::constraints::ConstraintEndpoint::Part {
                part_id: "right/HEADER".into(),
            },
            relation: crate::constraints::ConstraintRelation::Offset {
                offset: Vec2 { x: -4.0, y: 9.0 },
                rotation: -0.0,
            },
        }
    );

    let removed = canonicalize(
        &document,
        &EditOperation::RemoveConstraint {
            id: "constraint/authored-on-mirror".into(),
        },
    )
    .unwrap();
    assert_eq!(
        removed.operation,
        EditOperation::RemoveConstraint {
            id: "constraint/authored-on-mirror".into(),
        }
    );
    assert_eq!(
        removed.prepared_constraints,
        [super::constraints::PreparedConstraintEdit::Remove {
            id: "constraint/authored-on-mirror".into(),
        }]
    );
}

#[test]
fn convert_offset_keeps_legacy_missing_target_precedence_for_mirrors() {
    let mut document = rotated_fixture();
    document.constraints.push(Constraint::Mirror {
        id: "constraint/missing-target".into(),
        source_part_id: "left/U1".into(),
        target_part_id: "does-not-exist".into(),
        axis: crate::model::MirrorAxis::Vertical,
        coordinate: 4.0,
    });
    let error = canonicalize(
        &document,
        &EditOperation::ConvertOffsetConstraint {
            constraint_id: "constraint/missing-target".into(),
            to: crate::model::OffsetAxes::Part,
        },
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("target is missing"), "{error}");
}

#[test]
fn convert_offset_prepares_the_same_part_axes_values_as_the_legacy_operation() {
    let mut document = rotated_fixture();
    document.constraints.push(Constraint::Offset {
        id: "constraint/convert-axes".into(),
        source_part_id: "right/U1".into(),
        target_part_id: "right/HEADER".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::ConvertOffsetConstraint {
            constraint_id: "constraint/convert-axes".into(),
            to: crate::model::OffsetAxes::Part,
        },
    )
    .unwrap();

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("ConvertOffsetConstraint must prepare its converted relation");
    };
    let crate::constraints::FramedConstraint::Framed {
        id,
        source,
        target,
        relation: crate::constraints::ConstraintRelation::PartOffset { offset, rotation },
    } = record
    else {
        panic!("the converted relation must remain a part-offset");
    };
    assert_eq!(id, "constraint/convert-axes");
    assert_eq!(
        source,
        &crate::constraints::ConstraintEndpoint::LinkedMirror {
            pair_id: "pair".into(),
            source_part_id: "left/U1".into(),
            mirror_part_id: "right/U1".into(),
        }
    );
    assert_eq!(
        target,
        &crate::constraints::ConstraintEndpoint::Part {
            part_id: "right/HEADER".into(),
        }
    );
    assert_eq!(offset.x, -5.0);
    assert!((offset.y - 2.0).abs() < 1e-12, "{offset:?}");
    assert_eq!(rotation.to_bits(), 15.0_f64.to_bits());
}

#[test]
fn set_constraint_compiles_mixed_linked_and_local_overlay_endpoints() {
    let document = rotated_fixture();
    let canonical = canonicalize(
        &document,
        &EditOperation::SetConstraint {
            constraint: Constraint::Offset {
                id: "constraint/mixed-local".into(),
                source_part_id: "right/U1".into(),
                target_part_id: "right/HEADER".into(),
                offset: Vec2 { x: 5.0, y: -2.0 },
                rotation: 15.0,
            },
        },
    )
    .unwrap();

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &canonical.prepared_constraints[0]
    else {
        panic!("the mixed SetConstraint must prepare one relation");
    };
    assert_eq!(
        record,
        &crate::constraints::FramedConstraint::Framed {
            id: "constraint/mixed-local".into(),
            source: crate::constraints::ConstraintEndpoint::LinkedMirror {
                pair_id: "pair".into(),
                source_part_id: "left/U1".into(),
                mirror_part_id: "right/U1".into(),
            },
            target: crate::constraints::ConstraintEndpoint::Part {
                part_id: "right/HEADER".into(),
            },
            relation: crate::constraints::ConstraintRelation::Offset {
                offset: Vec2 { x: 5.0, y: -2.0 },
                rotation: 15.0,
            },
        }
    );
}

#[test]
fn set_constraint_refuses_an_omitted_legacy_endpoint() {
    let document = rotated_fixture();
    let error = canonicalize(
        &document,
        &EditOperation::SetConstraint {
            constraint: Constraint::Offset {
                id: "constraint/omitted".into(),
                source_part_id: "left/U1".into(),
                target_part_id: "right/GONE".into(),
                offset: Vec2 { x: 1.0, y: 2.0 },
                rotation: 3.0,
            },
        },
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("omitted"), "{error}");
    assert!(error.contains("right/GONE"), "{error}");
}

#[test]
fn moving_a_linked_target_retunes_the_offset_in_its_authored_target_view() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.push(Constraint::Offset {
        id: "constraint/linked-target".into(),
        source_part_id: "right/HEADER".into(),
        target_part_id: "right/U1".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "left/U1".into(),
                at: Vec2 { x: -20.0, y: 4.0 },
            }],
        },
    )
    .unwrap();

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("the linked target move must prepare one retuned record");
    };
    let crate::constraints::FramedConstraint::Framed {
        id,
        source,
        target,
        relation: crate::constraints::ConstraintRelation::Offset { offset, rotation },
    } = record
    else {
        panic!("the target retune must retain the Offset relation");
    };
    assert_eq!(id, "constraint/linked-target");
    assert_eq!(
        source,
        &crate::constraints::ConstraintEndpoint::Part {
            part_id: "right/HEADER".into(),
        }
    );
    assert_eq!(
        target,
        &crate::constraints::ConstraintEndpoint::LinkedMirror {
            pair_id: "pair".into(),
            source_part_id: "left/U1".into(),
            mirror_part_id: "right/U1".into(),
        }
    );
    // The rotated pair maps the old and requested authoritative poses into the authored target
    // view; their literal visible delta is (-4, -4), added to the stored (5, -2).
    assert!((offset.x - 1.0).abs() < 1e-12, "{offset:?}");
    assert!((offset.y + 6.0).abs() < 1e-12, "{offset:?}");
    assert_eq!(rotation.to_bits(), 15.0_f64.to_bits());
}

#[test]
fn moving_a_visible_mirror_constraint_target_uses_its_literal_inverse_position() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.push(Constraint::Offset {
        id: "constraint/raw-mirror-target".into(),
        source_part_id: "left/HEADER".into(),
        target_part_id: "right/U1".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "right/U1".into(),
                at: Vec2 { x: 104.0, y: -1.0 },
            }],
        },
    )
    .unwrap();

    let EditOperation::MoveParts { positions } = &prepared.operation else {
        panic!("a visible mirror target move must stay a MoveParts operation");
    };
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].id, "left/U1");
    assert!(
        (positions[0].at.x + 29.0).abs() < 1e-12,
        "{:?}",
        positions[0]
    );
    assert!(
        (positions[0].at.y + 4.0).abs() < 1e-12,
        "{:?}",
        positions[0]
    );
    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("moving the visible mirror target must retune the same global record");
    };
    let crate::constraints::FramedConstraint::Framed {
        id,
        relation: crate::constraints::ConstraintRelation::Offset { offset, rotation },
        ..
    } = record
    else {
        panic!("the authored Offset relation must remain an Offset");
    };
    assert_eq!(id, "constraint/raw-mirror-target");
    // The pair map sends the original source (-24, 0) to (100, -6); the visible
    // request (104, -1) contributes (4, 5) to the legacy stored (5, -2).
    assert_eq!(*offset, Vec2 { x: 9.0, y: 3.0 });
    assert_eq!(rotation.to_bits(), 15.0_f64.to_bits());
}

#[test]
fn moving_a_part_offset_target_uses_the_linked_source_view_and_keeps_position_heading() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.push(Constraint::PartOffset {
        id: "constraint/part-view-target".into(),
        source_part_id: "right/U1".into(),
        target_part_id: "right/HEADER".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "right/HEADER".into(),
                at: Vec2 { x: 228.0, y: 32.0 },
            }],
        },
    )
    .unwrap();

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("the PartOffset target move must prepare one retuned record");
    };
    let crate::constraints::FramedConstraint::Framed {
        relation: crate::constraints::ConstraintRelation::PartOffset { offset, rotation },
        ..
    } = record
    else {
        panic!("the PartOffset relation must stay in its authored axes");
    };
    // Source view heading is 270 degrees for the 90-degree pair map, so (4, 2) turns back to
    // (-2, 4) in the relation's source axes.
    assert!((offset.x - 3.0).abs() < 1e-12, "{offset:?}");
    assert!((offset.y - 2.0).abs() < 1e-12, "{offset:?}");
    assert_eq!(rotation.to_bits(), 15.0_f64.to_bits());
}

#[test]
fn moving_source_and_target_in_one_batch_retunes_by_target_delta_not_final_fit() {
    let mut document = rotated_fixture();
    document.constraints.push(Constraint::Offset {
        id: "constraint/batch".into(),
        source_part_id: "right/HEADER".into(),
        target_part_id: "right/FREE".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: -0.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![
                crate::model::Position {
                    id: "right/FREE".into(),
                    at: Vec2 { x: 302.0, y: 4.0 },
                },
                crate::model::Position {
                    id: "right/HEADER".into(),
                    at: Vec2 { x: 225.0, y: 31.0 },
                },
            ],
        },
    )
    .unwrap();

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("the batch target move must prepare one retuned record");
    };
    let crate::constraints::FramedConstraint::Framed {
        relation: crate::constraints::ConstraintRelation::Offset { offset, rotation },
        ..
    } = record
    else {
        panic!("the batch must retain Offset");
    };
    // Existing MoveParts semantics add only target's requested delta (2, 4). A fit against the
    // moved source would replace this value with the target's final source-relative position.
    assert_eq!(*offset, Vec2 { x: 7.0, y: 2.0 });
    assert_eq!(rotation.to_bits(), (-0.0_f64).to_bits());

    let reversed = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![
                crate::model::Position {
                    id: "right/HEADER".into(),
                    at: Vec2 { x: 225.0, y: 31.0 },
                },
                crate::model::Position {
                    id: "right/FREE".into(),
                    at: Vec2 { x: 302.0, y: 4.0 },
                },
            ],
        },
    )
    .unwrap();
    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &reversed.prepared_constraints[0]
    else {
        panic!("the reordered batch must retain one retuned record");
    };
    let crate::constraints::FramedConstraint::Framed {
        relation: crate::constraints::ConstraintRelation::Offset { offset, rotation },
        ..
    } = record
    else {
        panic!("the reordered batch must retain Offset");
    };
    assert_eq!(*offset, Vec2 { x: 7.0, y: 2.0 });
    assert_eq!(rotation.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn moving_a_legacy_mirror_constraint_target_is_refused() {
    let mut document = rotated_fixture();
    document.constraints.push(Constraint::Mirror {
        id: "constraint/mirror-target".into(),
        source_part_id: "right/HEADER".into(),
        target_part_id: "right/FREE".into(),
        axis: crate::model::MirrorAxis::Vertical,
        coordinate: 200.0,
    });
    let error = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "right/FREE".into(),
                at: Vec2 { x: 302.0, y: 4.0 },
            }],
        },
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("mirror"), "{error}");
    assert!(error.contains("drag"), "{error}");
}

#[test]
fn moving_a_companion_target_defers_until_its_key_owner_pose_is_resolved() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.push(Constraint::Offset {
        id: "constraint/companion-target".into(),
        source_part_id: "left/U1".into(),
        target_part_id: "matrix/right-keys/r0c0/diode".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "matrix/right-keys/r0c0/diode".into(),
                at: Vec2 { x: 187.0, y: -4.0 },
            }],
        },
    )
    .unwrap();

    let [super::constraints::PreparedConstraintEdit::DeferredTargetRetune(intent)] =
        prepared.prepared_constraints.as_slice()
    else {
        panic!("a companion retune must wait for its live key owner pose");
    };
    let crate::constraints::FramedConstraint::Framed { id, .. } = &intent.record;
    assert_eq!(id, "constraint/companion-target");
    assert_eq!(
        intent.target_owner.as_deref(),
        Some("matrix/left-keys/r0c0")
    );
    assert_eq!(intent.source_owner, None);
    assert_eq!(intent.requested.id, "matrix/left-keys/r0c0/diode");
}

#[test]
fn moving_a_linked_key_and_companion_in_either_order_keeps_the_owner_intent() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.push(Constraint::Offset {
        id: "constraint/batched-companion".into(),
        source_part_id: "left/U1".into(),
        target_part_id: "matrix/right-keys/r0c0/diode".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let key = crate::model::Position {
        id: "matrix/right-keys/r0c0".into(),
        at: Vec2 { x: 190.0, y: -5.0 },
    };
    let companion = crate::model::Position {
        id: "matrix/right-keys/r0c0/diode".into(),
        at: Vec2 { x: 198.0, y: -7.0 },
    };
    let key_then_companion = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![key.clone(), companion.clone()],
        },
    )
    .unwrap();
    let companion_then_key = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![companion, key],
        },
    )
    .unwrap();

    for prepared in [&key_then_companion, &companion_then_key] {
        let EditOperation::MoveParts { positions } = &prepared.operation else {
            panic!("the linked key and companion moves must remain in the canonical batch");
        };
        assert_eq!(positions.len(), 2);
        let source_key = positions
            .iter()
            .find(|position| position.id == "matrix/left-keys/r0c0")
            .unwrap();
        let source_companion = positions
            .iter()
            .find(|position| position.id == "matrix/left-keys/r0c0/diode")
            .unwrap();
        // The independently worked inverse of the 90-degree map is
        // (-30 - y, 100 - x): key (-25, -90), companion (-23, -98).
        assert!(
            (source_key.at.x + 25.0).abs() < 1e-12,
            "{:?}",
            source_key.at
        );
        assert!(
            (source_key.at.y + 90.0).abs() < 1e-12,
            "{:?}",
            source_key.at
        );
        assert!(
            (source_companion.at.x + 23.0).abs() < 1e-12,
            "{:?}",
            source_companion.at
        );
        assert!(
            (source_companion.at.y + 98.0).abs() < 1e-12,
            "{:?}",
            source_companion.at
        );

        let [super::constraints::PreparedConstraintEdit::DeferredTargetRetune(intent)] =
            prepared.prepared_constraints.as_slice()
        else {
            panic!("the companion stays deferred until its key owner is resolved");
        };
        assert_eq!(
            intent.target_owner.as_deref(),
            Some("matrix/left-keys/r0c0")
        );
        assert_eq!(intent.requested.id, "matrix/left-keys/r0c0/diode");
        assert_eq!(intent.requested.at, source_companion.at);
    }
}

#[test]
fn moving_a_duplicate_position_companion_in_either_order_uses_its_requested_key_owner() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .companion_mode = crate::model::MirroredCompanionMode::DuplicatePosition;
    document.constraints.push(Constraint::Offset {
        id: "constraint/duplicate-position-child".into(),
        source_part_id: "left/U1".into(),
        target_part_id: "matrix/right-keys/r0c0/diode".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let key = crate::model::Position {
        id: "matrix/right-keys/r0c0".into(),
        at: Vec2 { x: 190.0, y: -5.0 },
    };
    let companion = crate::model::Position {
        id: "matrix/right-keys/r0c0/diode".into(),
        at: Vec2 { x: 198.0, y: -7.0 },
    };
    let key_then_companion = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![key.clone(), companion.clone()],
        },
    )
    .unwrap();
    let companion_then_key = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![companion, key],
        },
    )
    .unwrap();

    for prepared in [&key_then_companion, &companion_then_key] {
        let EditOperation::MoveParts { positions } = &prepared.operation else {
            panic!("the duplicate-position move must retain its ordered batch");
        };
        let source_key = positions
            .iter()
            .find(|position| position.id == "matrix/left-keys/r0c0")
            .unwrap();
        let source_companion = positions
            .iter()
            .find(|position| position.id == "matrix/left-keys/r0c0/diode")
            .unwrap();
        // DuplicatePosition carries the mirror-local companion delta (8, -2) onto
        // the source key at (-25, -90), giving source companion (-17, -92).
        assert!(
            (source_key.at.x + 25.0).abs() < 1e-12,
            "{:?}",
            source_key.at
        );
        assert!(
            (source_key.at.y + 90.0).abs() < 1e-12,
            "{:?}",
            source_key.at
        );
        assert!(
            (source_companion.at.x + 17.0).abs() < 1e-12,
            "{:?}",
            source_companion.at
        );
        assert!(
            (source_companion.at.y + 92.0).abs() < 1e-12,
            "{:?}",
            source_companion.at
        );

        let [super::constraints::PreparedConstraintEdit::DeferredTargetRetune(intent)] =
            prepared.prepared_constraints.as_slice()
        else {
            panic!("the duplicate-position child must wait for its requested key owner");
        };
        assert_eq!(
            intent.target_owner.as_deref(),
            Some("matrix/left-keys/r0c0")
        );
        assert_eq!(intent.requested.id, "matrix/left-keys/r0c0/diode");
        assert_eq!(intent.requested.at, source_companion.at);
    }
}

#[test]
fn moving_a_companion_with_a_constrained_key_keeps_both_prepared_intents() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.extend([
        Constraint::Offset {
            id: "constraint/key-owner".into(),
            source_part_id: "left/U1".into(),
            target_part_id: "matrix/right-keys/r0c0".into(),
            offset: Vec2 { x: 2.0, y: 1.0 },
            rotation: -0.0,
        },
        Constraint::Offset {
            id: "constraint/constrained-owner-child".into(),
            source_part_id: "left/U1".into(),
            target_part_id: "matrix/right-keys/r0c0/diode".into(),
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        },
    ]);
    let key = crate::model::Position {
        id: "matrix/right-keys/r0c0".into(),
        at: Vec2 { x: 190.0, y: -5.0 },
    };
    let companion = crate::model::Position {
        id: "matrix/right-keys/r0c0/diode".into(),
        at: Vec2 { x: 198.0, y: -7.0 },
    };
    for positions in [vec![key.clone(), companion.clone()], vec![companion, key]] {
        let prepared = canonicalize(&document, &EditOperation::MoveParts { positions }).unwrap();
        let mut upsert_ids = Vec::new();
        let mut deferred = Vec::new();
        for edit in &prepared.prepared_constraints {
            match edit {
                super::constraints::PreparedConstraintEdit::Upsert(
                    crate::constraints::FramedConstraint::Framed { id, .. },
                ) => upsert_ids.push(id.as_str()),
                super::constraints::PreparedConstraintEdit::DeferredTargetRetune(intent) => {
                    deferred.push(intent)
                }
                other => panic!("unexpected prepared edit: {other:?}"),
            }
        }
        assert_eq!(upsert_ids, ["constraint/key-owner"]);
        assert_eq!(deferred.len(), 1);
        assert_eq!(
            deferred[0].target_owner.as_deref(),
            Some("matrix/left-keys/r0c0")
        );
        assert_eq!(deferred[0].requested.id, "matrix/left-keys/r0c0/diode");
    }
}

// ---- coverage ------------------------------------------------------------------------------

#[test]
fn deliberately_unchanged_and_delegated_requests_run_as_written() {
    for request in [
        json!({"kind": "set-key-binding", "boardId": "right", "keyId": "right/U1", "binding": "A"}),
        json!({"kind": "apply-mechanical-edit", "edit": {"target": {"kind": "instance", "instanceId": "left"}, "change": "enable"}}),
    ] {
        expect_operation(&fixture(), request.clone(), request);
    }
}

#[test]
fn independent_case_bodies_and_adr_excluded_bindings_keep_targeted_semantics() {
    let document = fixture();
    let case = op(json!({"kind":"set-case","body":{"id":"right/shell",
        "name":"Right shell","boardId":"right","kind":"tray","thickness":2.0,
        "clearance":0.2}}));
    let case_result = canonicalize(&document, &case).unwrap();
    assert_eq!(
        case_result.coverage,
        Coverage::DeliberatelyUnchanged("case bodies are independently authored per board")
    );
    assert_eq!(case_result.operation, case);

    for binding in [
        op(json!({"kind":"set-key-binding","boardId":"right","keyId":"right/U1","binding":"A"})),
        op(
            json!({"kind":"edit-keymap","change":{"kind":"add-layer","id":"right-layer","name":"Right"}}),
        ),
    ] {
        let result = canonicalize(&document, &binding).unwrap();
        assert_eq!(
            result.coverage,
            Coverage::DeliberatelyUnchanged(
                "bindings are never reflected; the edit keeps its targeted-half semantics (ADR-0010)"
            )
        );
        assert_eq!(result.operation, binding);
    }
}

#[test]
fn pair_creation_mechanical_lifecycle_half_creation_and_replace_are_passed_through() {
    let document = fixture();
    let operations = [
        (
            op(json!({"kind":"create-mirrored-pair",
                "left":{"id":"left-layout","name":"Left","boardId":"left",
                    "matrixIds": ["left-matrix"],"partIds":[]},
                "right":{"id":"right-layout","name":"Right","boardId":"left",
                    "matrixIds": ["right-matrix"],"partIds":[],
                    "mirrorLink":{"sourceId":"left-layout","axisX":100.0}},
                "matrix":{"id":"left-matrix","rows":0,"columns":0,
                    "pitch":{"x":19.0,"y":19.0},"origin":{"x":0.0,"y":0.0},
                    "definitionId":"switch","partIds":[],"boardId":"left"}})),
            Coverage::Delegated(
                "CreateMirroredPair creates both linked layouts in one Core operation",
            ),
        ),
        (
            op(
                json!({"kind":"apply-mechanical-edit","edit":{"target":{"kind":"instance",
                "instanceId":"left"},"change":"enable"}}),
            ),
            Coverage::Delegated(
                "the typed mechanical lifecycle already edits both halves of a pair; inverting it again would apply it twice",
            ),
        ),
        (
            op(json!({"kind":"create-mirrored-half","pairId":"new-pair",
                "sourceInstanceId":"left",
                "mode":{"kind":"new-board","boardId":"right-new","instanceId":"right-instance"},
                "mapping":{"at":{"x":0.0,"y":0.0},"rotation":0.0},
                "idRule":"board-prefix","companionMode":"literal-mirror"})),
            Coverage::Delegated(
                "CreateMirroredHalf derives both halves through the Core mirror lifecycle",
            ),
        ),
        (
            EditOperation::ReplaceDocument {
                document: Box::new(ProjectDoc::empty("replacement", "Replacement")),
            },
            Coverage::Delegated(
                "ticket 116 owns source-authoritative Open and ReplaceDocument repair",
            ),
        ),
    ];
    for (operation, expected_coverage) in operations {
        let result = canonicalize(&document, &operation).unwrap();
        assert_eq!(result.coverage, expected_coverage);
        assert_eq!(result.operation, operation);
    }
}

#[test]
fn set_layout_on_native_mirror_maps_linked_members_and_preserves_hidden_and_local_members() {
    let document = native_layout_fixture();
    let request = EditOperation::SetLayout {
        layout: document
            .layouts
            .iter()
            .find(|layout| layout.id == "right-layout")
            .unwrap()
            .clone(),
    };
    let result = canonicalize(&document, &request).expect("native mirror layout translates");
    let EditOperation::SetLayout { layout } = result.operation else {
        panic!("SetLayout remains a SetLayout")
    };
    assert_eq!(layout.id, "left-layout");
    assert_eq!(layout.board_id, "left");
    assert_eq!(layout.matrix_ids, ["left-keys"]);
    assert_eq!(layout.part_ids, ["left/U1", "left/HEADER", "left/GONE"]);
}

#[test]
fn set_layout_native_mirror_reorders_projected_local_and_linked_members() {
    let document = native_layout_fixture();
    let mut requested = document.layouts[1].clone();
    requested.part_ids = vec![
        "right/HEADER".into(),
        "right/U1".into(),
        "right/FREE".into(),
    ];
    let result = canonicalize(&document, &EditOperation::SetLayout { layout: requested })
        .expect("a projected-member reorder can be expressed by source order");
    let EditOperation::SetLayout { layout } = result.operation else {
        panic!("SetLayout remains a SetLayout")
    };
    assert_eq!(layout.id, "left-layout");
    assert_eq!(layout.part_ids, ["left/HEADER", "left/U1", "left/GONE"]);
}

#[test]
fn set_layout_native_mirror_removes_linked_members_from_source_membership() {
    let document = native_layout_fixture();
    let mut requested = document.layouts[1].clone();
    requested.part_ids = vec!["right/HEADER".into(), "right/FREE".into()];
    let result = canonicalize(&document, &EditOperation::SetLayout { layout: requested })
        .expect("removing a linked member maps to source membership");
    let EditOperation::SetLayout { layout } = result.operation else {
        panic!("SetLayout remains a SetLayout")
    };
    assert_eq!(layout.id, "left-layout");
    assert_eq!(layout.part_ids, ["left/HEADER", "left/GONE"]);
}

#[test]
fn native_mirror_target_owned_name_and_membership_edits_pass_through_deliberately() {
    let document = native_layout_fixture();
    let cases = [
        (document.clone(), {
            let mut layout = document.layouts[1].clone();
            layout.name = "Mirror-specific name".into();
            layout
        }),
        (document.clone(), {
            let mut layout = document.layouts[1].clone();
            layout.part_ids.retain(|id| id != "right/FREE");
            layout
        }),
        (
            {
                let mut document = document.clone();
                document.layouts[1].part_ids.retain(|id| id != "right/FREE");
                document
            },
            {
                let mut layout = document.layouts[1].clone();
                layout.part_ids.retain(|id| id != "right/FREE");
                layout.part_ids.push("right/FREE".into());
                layout
            },
        ),
    ];
    for (document, layout) in cases {
        let request = EditOperation::SetLayout { layout };
        let result = canonicalize(&document, &request).expect("target-owned edit passes through");
        assert_eq!(result.operation, request);
        assert!(matches!(
            result.coverage,
            Coverage::DeliberatelyUnchanged(_)
        ));
    }
}

#[test]
fn source_local_and_legacy_only_layout_requests_keep_their_authored_targets() {
    let document = native_layout_fixture();
    let mut source = document
        .layouts
        .iter()
        .find(|layout| layout.id == "left-layout")
        .unwrap()
        .clone();
    source.name = "Source renamed".into();
    let source_request = EditOperation::SetLayout { layout: source };
    assert_eq!(
        canonicalize(&document, &source_request).unwrap().operation,
        source_request
    );

    let mut local_document = document.clone();
    local_document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(
            serde_json::from_value(json!({"kind":"layout",
            "mirrorId":"right-layout","state":"local"}))
            .unwrap(),
        );
    let mut local = local_document.layouts[1].clone();
    local.name = "Local target name".into();
    let local_request = EditOperation::SetLayout { layout: local };
    assert_eq!(
        canonicalize(&local_document, &local_request)
            .unwrap()
            .operation,
        local_request
    );

    let mut legacy_document = fixture();
    add_empty_local_matrix(&mut legacy_document, "legacy-source-matrix");
    add_empty_local_matrix(&mut legacy_document, "legacy-target-matrix");
    legacy_document.layouts = vec![
        serde_json::from_value(json!({"id":"legacy-source","name":"Legacy source",
            "boardId":"right","matrixIds": ["legacy-source-matrix"],"partIds":[]}))
        .unwrap(),
        serde_json::from_value(json!({"id":"legacy-mirror","name":"Legacy mirror",
            "boardId":"right","matrixIds": ["legacy-target-matrix"],"partIds":[],
            "mirrorLink":{"sourceId":"legacy-source","axisX":100.0}}))
        .unwrap(),
    ];
    let mut legacy_target = legacy_document.layouts[1].clone();
    legacy_target.name = "Legacy target renamed".into();
    let legacy_request = EditOperation::SetLayout {
        layout: legacy_target,
    };
    assert_eq!(
        canonicalize(&legacy_document, &legacy_request)
            .unwrap()
            .operation,
        legacy_request
    );
}

#[test]
fn native_layout_refuses_unrepresentable_name_local_membership_and_legacy_overlap() {
    let document = native_layout_fixture();
    let mut renamed = document.layouts[1].clone();
    renamed.name = "Target-only rename".into();
    let rename_request = EditOperation::SetLayout { layout: renamed };
    let renamed_result = canonicalize(&document, &rename_request).unwrap();
    assert_eq!(renamed_result.operation, rename_request);
    assert!(matches!(
        renamed_result.coverage,
        Coverage::DeliberatelyUnchanged(_)
    ));

    let mut reassigned = document.layouts[1].clone();
    reassigned.matrix_ids = vec!["left-keys".into()];
    assert!(matches!(
        canonicalize(&document, &EditOperation::SetLayout { layout: reassigned }),
        Err(Refusal::Request(message)) if message.contains("ownership")
    ));

    let mut linked = document.layouts[1].clone();
    linked.mirror_link = Some(
        serde_json::from_value(json!({
            "sourceId":"left-layout","axisX":100.0
        }))
        .unwrap(),
    );
    assert!(matches!(
        canonicalize(&document, &EditOperation::SetLayout { layout: linked }),
        Err(Refusal::Request(message)) if message.contains("legacy mirror link")
    ));

    let mut source_with_new_legacy_link = document.layouts[0].clone();
    source_with_new_legacy_link.mirror_link =
        Some(serde_json::from_value(json!({"sourceId":"right-layout","axisX":100.0})).unwrap());
    assert!(matches!(
        canonicalize(
            &document,
            &EditOperation::SetLayout {
                layout: source_with_new_legacy_link
            }
        ),
        Err(Refusal::Request(message)) if message.contains("native mirrored-pair")
    ));

    let mut new_overlap = document.clone();
    add_empty_local_matrix(&mut new_overlap, "new-legacy-peer-matrix");
    new_overlap.layouts.push(
        serde_json::from_value(json!({"id":"new-legacy-peer","name":"New legacy peer",
            "boardId":"right","matrixIds": ["new-legacy-peer-matrix"],"partIds":[]}))
        .unwrap(),
    );
    let mut new_legacy_link = new_overlap.layouts[2].clone();
    new_legacy_link.mirror_link =
        Some(serde_json::from_value(json!({"sourceId":"right-layout","axisX":100.0})).unwrap());
    assert!(matches!(
        canonicalize(
            &new_overlap,
            &EditOperation::SetLayout {
                layout: new_legacy_link
            }
        ),
        Err(Refusal::Request(message)) if message.contains("native mirrored-pair")
    ));

    let mut changed_local_members = document.layouts[1].clone();
    changed_local_members
        .part_ids
        .retain(|id| id != "right/FREE");
    let local_request = EditOperation::SetLayout {
        layout: changed_local_members,
    };
    let local_result = canonicalize(&document, &local_request).unwrap();
    assert_eq!(local_result.operation, local_request);
    assert!(matches!(
        local_result.coverage,
        Coverage::DeliberatelyUnchanged(_)
    ));

    let mut removed_source_local = document.layouts[1].clone();
    removed_source_local
        .part_ids
        .retain(|id| id != "right/HEADER");
    assert!(matches!(
        canonicalize(
            &document,
            &EditOperation::SetLayout {
                layout: removed_source_local
            }
        ),
        Err(Refusal::Request(message)) if message.contains("cannot be removed")
    ));

    let mut mixed_target_change = document.layouts[1].clone();
    mixed_target_change.name = "Mixed target-owned name".into();
    mixed_target_change.part_ids = vec![
        "right/HEADER".into(),
        "right/U1".into(),
        "right/FREE".into(),
    ];
    assert!(matches!(
        canonicalize(
            &document,
            &EditOperation::SetLayout {
                layout: mixed_target_change
            }
        ),
        Err(Refusal::Request(message)) if message.contains("source-projected membership changes")
    ));

    let mut overlap = document.clone();
    add_empty_local_matrix(&mut overlap, "right-local-matrix");
    overlap.layouts.push(
        serde_json::from_value(json!({"id":"legacy-peer","name":"Legacy peer",
            "boardId":"right","matrixIds": ["right-local-matrix"],"partIds":[],
            "mirrorLink":{"sourceId":"right-layout","axisX":100.0}}))
        .unwrap(),
    );
    for id in ["right-layout", "left-layout"] {
        let operation = EditOperation::SetLayout {
            layout: overlap
                .layouts
                .iter()
                .find(|layout| layout.id == id)
                .unwrap()
                .clone(),
        };
        let result = canonicalize(&overlap, &operation);
        assert!(
            matches!(
                &result,
                Err(Refusal::Request(message)) if message.contains("native mirrored-pair")
                    && message.contains("legacy layout mirror link")
            ),
            "{operation:?}: {result:?}"
        );
    }

    let mut repair = overlap
        .layouts
        .iter()
        .find(|layout| layout.id == "legacy-peer")
        .unwrap()
        .clone();
    repair.mirror_link = None;
    let repair_request = EditOperation::SetLayout { layout: repair };
    assert_eq!(
        canonicalize(&overlap, &repair_request).unwrap().operation,
        repair_request
    );

    let mut direct_overlap = document.clone();
    add_empty_local_matrix(&mut direct_overlap, "native-legacy-source-matrix");
    direct_overlap.layouts.push(
        serde_json::from_value(json!({"id":"native-legacy-source",
            "name":"Native legacy source","boardId":"right",
            "matrixIds": ["native-legacy-source-matrix"],"partIds":[]}))
        .unwrap(),
    );
    let native_mirror = direct_overlap
        .layouts
        .iter_mut()
        .find(|layout| layout.id == "right-layout")
        .unwrap();
    native_mirror.mirror_link = Some(
        serde_json::from_value(json!({"sourceId":"native-legacy-source","axisX":100.0})).unwrap(),
    );
    let mut direct_repair = native_mirror.clone();
    direct_repair.mirror_link = None;
    let mut changed_while_unlinking = direct_repair.clone();
    changed_while_unlinking.name = "Changed during unlink".into();
    assert!(matches!(
        canonicalize(
            &direct_overlap,
            &EditOperation::SetLayout {
                layout: changed_while_unlinking
            }
        ),
        Err(Refusal::Request(message)) if message.contains("native mirrored-pair")
    ));
    let direct_repair_request = EditOperation::SetLayout {
        layout: direct_repair,
    };
    assert_eq!(
        canonicalize(&direct_overlap, &direct_repair_request)
            .unwrap()
            .operation,
        direct_repair_request
    );
}

#[test]
fn omitted_native_layout_is_refused_instead_of_recreated() {
    let mut document = native_layout_fixture();
    let omitted_request = EditOperation::SetLayout {
        layout: document.layouts[1].clone(),
    };
    let mut source_request = document.layouts[0].clone();
    source_request.name = "Source edit while mirror omitted".into();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(
            serde_json::from_value(json!({"kind":"layout",
            "mirrorId":"right-layout","state":"omitted"}))
            .unwrap(),
        );
    document
        .layouts
        .retain(|layout| layout.id != "right-layout");
    assert_eq!(
        canonicalize(
            &document,
            &EditOperation::SetLayout {
                layout: source_request.clone()
            }
        )
        .unwrap()
        .operation,
        EditOperation::SetLayout {
            layout: source_request
        }
    );
    assert!(matches!(
        canonicalize(&document, &omitted_request),
        Err(Refusal::Request(message)) if message.contains("omitted")
    ));
}

#[test]
fn set_outline_on_a_linked_mirror_maps_feature_geometry_and_identity_back_to_source() {
    let mut document = fixture();
    document.boards[0].outline_ids.push("left/cutout".into());
    document.boards[1].outline_ids.push("right/cutout".into());
    document.outline.extend([
        serde_json::from_value(
            json!({"kind": "polygon", "id": "left/cutout", "anchorPartId": null,
            "points": [{"x": 190, "y": 4}, {"x": 195, "y": 4}, {"x": 195, "y": 8}],
            "operation": "subtract"}),
        )
        .unwrap(),
        serde_json::from_value(
            json!({"kind": "polygon", "id": "right/cutout", "anchorPartId": null,
            "points": [{"x": 5, "y": 8}, {"x": 5, "y": 4}, {"x": 10, "y": 4}],
            "operation": "subtract"}),
        )
        .unwrap(),
    ]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(serde_json::from_value(corr("outline", "left/cutout", "right/cutout")).unwrap());
    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"},
        "change": {"set-feature": {"feature": {"kind": "polygon", "id": "right/cutout",
            "anchorPartId": null, "points": [{"x": 3, "y": 9}, {"x": 3, "y": 6}, {"x": 7, "y": 6}],
            "operation": "subtract"}}},
        "seed": 5, "reviewed": null}});
    let translated =
        canonical(&document, &request).expect("a linked mirror outline edit translates");
    assert_eq!(translated.coverage, Coverage::Translated);
    assert_close(
        &serde_json::to_value(translated.operation).unwrap(),
        &json!({"kind": "apply-outline-edit", "edit": {
            "target": {"kind": "board", "boardId": "left"},
            "change": {"set-feature": {"feature": {"kind": "polygon", "id": "left/cutout",
                "points": [{"x": 193, "y": 6}, {"x": 197, "y": 6}, {"x": 197, "y": 9}],
                "operation": "subtract"}}},
            "seed": 5}}),
        "operation",
    );
}

#[test]
fn every_outline_change_arm_is_classified_and_keeps_the_targeted_scalar_or_lifecycle_value() {
    let changes = [
        json!({"set-margin": {"margin": 3.0}}),
        json!({"set-corners": {"corners": "fillet"}}),
        json!({"set-size": {"size": 4.0}}),
        json!({"set-bridge-width": {"bridge_width": 5.0}}),
        json!({"set-repair-enabled": {"enabled": false}}),
        json!({"set-maximum-gap-span": {"value": 6.0}}),
        json!({"set-minimum-connection-width": {"value": 1.5}}),
        json!({"set-edge-clearance": {"value": 0.25}}),
        json!({"set-protected-gap": {"gap_id": "right/gap", "points": []}}),
        json!({"remove-protected-gap": {"gap_id": "right/gap"}}),
        json!({"add-feature": {"feature": {"kind": "polygon", "id": "right/new", "points": [], "operation": "add"}}}),
        json!({"set-feature": {"feature": {"kind": "polygon", "id": "right/old", "points": [], "operation": "add"}}}),
        json!({"remove-feature": {"feature_id": "right/old"}}),
        json!({"add-connection": {"connection": {"id": "right/connection", "width": 2.0, "points": []}}}),
        json!({"remove-connection": {"connection_id": "right/connection"}}),
        json!({"create-generated": {"margin": 2.0, "settings": {}, "part_ids": ["right/U1"]}}),
        json!({"create-linked-refinement": {"version_id": "right-version", "name": "Linked"}}),
        json!({"copy": {"version_id": "right-version", "name": "Fixed"}}),
        json!({"freeze": {"name": "Frozen"}}),
        json!({"edit-generated-perimeter": {"contour": 2, "points": []}}),
        json!({"edit-feature-perimeter": {"feature_id": "right/old", "points": []}}),
        json!("activate"),
        json!({"rename": {"name": "Renamed"}}),
        json!("delete"),
    ];
    assert_eq!(changes.len(), 24);
    for change in changes {
        let is_linked_refinement = change.get("create-linked-refinement").is_some();
        let request = json!({"kind": "apply-outline-edit", "edit": {
            "target": {"kind": "board", "boardId": "right"}, "change": change,
            "seed": 77, "reviewed": null
        }});
        let result = canonical(&fixture(), &request).expect("every outline change is classified");
        assert_eq!(result.coverage, Coverage::Translated);
        let result = serde_json::to_value(result.operation).unwrap();
        assert_eq!(result["edit"]["target"]["boardId"], "left");
        assert_eq!(
            result["edit"]["seed"], 77,
            "captured allocation seed is preserved"
        );
        if is_linked_refinement {
            assert_eq!(
                result["edit"]["change"]["create-linked-refinement"]["version_id"],
                "left-version"
            );
        }
    }
}

#[test]
fn rotated_outline_controls_anchors_reviews_and_identity_are_inverse_mapped_together() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend([
            serde_json::from_value(corr("outline-version", "left-version", "right-version"))
                .unwrap(),
            serde_json::from_value(corr("outline", "left/feature", "right/feature")).unwrap(),
        ]);
    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "version", "boardId": "right", "versionId": "right-version"},
        "change": {"set-feature": {"feature": {"kind": "part-envelope", "id": "right/feature",
            "partIds": ["right/U1", "matrix/right-keys/r0c0", "right/FREE"], "margin": 2.0, "operation": "add",
            "connections": [{"id": "right/connection", "width": 2.0, "points": [
                {"at": {"x": 80, "y": -25}, "partId": null},
                {"at": {"x": 4, "y": 5}, "partId": "right/U1"}]}]}}},
        "seed": 4, "reviewed": [{"points": [{"x": 80, "y": -25}, {"x": 70, "y": -20}, {"x": 65, "y": -18}], "hole": false}]
    }});
    let translated = canonical(&document, &request).expect("rotated reviewed outline translates");
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_eq!(value["edit"]["target"]["boardId"], "left");
    assert_eq!(value["edit"]["target"]["versionId"], "left-version");
    assert_eq!(
        value["edit"]["change"]["set-feature"]["feature"]["id"],
        "left/feature"
    );
    assert_eq!(
        value["edit"]["change"]["set-feature"]["feature"]["partIds"],
        json!(["left/U1", "matrix/left-keys/r0c0", "right/FREE"])
    );
    assert_close(
        &value["edit"]["change"]["set-feature"]["feature"]["connections"][0]["points"][0]["at"],
        &json!({"x": -5.0, "y": 20.0}),
        "world control",
    );
    assert_eq!(
        value["edit"]["change"]["set-feature"]["feature"]["connections"][0]["points"][1]["at"],
        json!({"x": 4.0, "y": -5.0})
    );
    assert_eq!(
        value["edit"]["change"]["set-feature"]["feature"]["connections"][0]["points"][1]["partId"],
        "left/U1"
    );
    assert_close(
        &value["edit"]["reviewed"][0]["points"][0],
        &json!({"x": -12.0, "y": 35.0}),
        "review first",
    );
    assert_close(
        &value["edit"]["reviewed"][0]["points"][2],
        &json!({"x": -5.0, "y": 20.0}),
        "review last",
    );
}

#[test]
fn an_anchor_in_another_matrix_uses_its_own_matrix_frame_rule() {
    let mut document = fixture();
    let source_id = "matrix/left-other/r0c0";
    let mirror_id = "matrix/right-other/r0c0";
    let mut source_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .clone();
    source_matrix.id = "left-other".into();
    source_matrix.part_ids = vec![source_id.into()];
    let mut mirror_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "right-keys")
        .unwrap()
        .clone();
    mirror_matrix.id = "right-other".into();
    mirror_matrix.part_ids = vec![mirror_id.into()];
    for (old_id, new_id, board_id) in [
        ("matrix/left-keys/r0c0", source_id, "left"),
        ("matrix/right-keys/r0c0", mirror_id, "right"),
    ] {
        let mut new_part = document
            .parts
            .iter()
            .find(|part| part.id == old_id)
            .unwrap()
            .clone();
        new_part.id = new_id.into();
        document.parts.push(new_part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .part_ids
            .push(new_id.into());
    }
    document.matrices.extend([source_matrix, mirror_matrix]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend([
            serde_json::from_value(corr("matrix", "left-other", "right-other")).unwrap(),
            serde_json::from_value(corr(
                "part",
                "matrix/left-other/r0c0",
                "matrix/right-other/r0c0",
            ))
            .unwrap(),
        ]);
    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"},
        "change": {"add-feature": {"feature": {"kind": "polygon",
            "id": "right/new", "anchorPartId": "matrix/right-other/r0c0",
            "points": [{"x": 4, "y": 6}, {"x": 8, "y": 10}, {"x": 12, "y": 14}],
            "operation": "add"}}},
        "seed": 13
    }});

    let translated = canonical(&document, &request).expect("the linked anchor translates");
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_eq!(
        value["edit"]["change"]["add-feature"]["feature"]["id"],
        "left/new"
    );
    assert_eq!(
        value["edit"]["change"]["add-feature"]["feature"]["anchorPartId"],
        "matrix/left-other/r0c0"
    );
    assert_eq!(
        value["edit"]["change"]["add-feature"]["feature"]["points"],
        json!([{"x": -12.0, "y": 14.0}, {"x": -8.0, "y": 10.0}, {"x": -4.0, "y": 6.0}])
    );
}

#[test]
fn a_matrix_shaped_anchor_without_matrix_membership_uses_full_heading_rule() {
    let mut document = fixture();
    let source_id = "matrix/left-loose/r0c0";
    let mirror_id = "matrix/right-loose/r0c0";
    for (old_id, new_id, board_id) in [
        ("matrix/left-keys/r0c0", source_id, "left"),
        ("matrix/right-keys/r0c0", mirror_id, "right"),
    ] {
        let mut new_part = document
            .parts
            .iter()
            .find(|part| part.id == old_id)
            .unwrap()
            .clone();
        new_part.id = new_id.into();
        document.parts.push(new_part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .part_ids
            .push(new_id.into());
    }
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(serde_json::from_value(corr("part", source_id, mirror_id)).unwrap());

    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"},
        "change": {"add-feature": {"feature": {"kind": "polygon",
            "id": "right/new", "anchorPartId": mirror_id,
            "points": [{"x": 4, "y": 6}, {"x": 8, "y": 10}, {"x": 12, "y": 14}],
            "operation": "add"}}},
        "seed": 17
    }});
    let translated = canonical(&document, &request).expect("the linked anchor translates");
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_eq!(
        value["edit"]["change"]["add-feature"]["feature"]["points"],
        json!([{"x": 12.0, "y": -14.0}, {"x": 8.0, "y": -10.0}, {"x": 4.0, "y": -6.0}])
    );
}

#[test]
fn legacy_matrix_membership_without_board_id_uses_matrix_frame_rule() {
    let mut document = fixture();
    let source_id = "matrix/left-legacy/r0c0";
    let mirror_id = "matrix/right-legacy/r0c0";
    let mut source_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .clone();
    source_matrix.id = "left-legacy".into();
    source_matrix.board_id = None;
    source_matrix.part_ids = vec![source_id.into()];
    let mut mirror_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "right-keys")
        .unwrap()
        .clone();
    mirror_matrix.id = "right-legacy".into();
    mirror_matrix.board_id = None;
    mirror_matrix.part_ids = vec![mirror_id.into()];
    for (old_id, new_id, board_id) in [
        ("matrix/left-keys/r0c0", source_id, "left"),
        ("matrix/right-keys/r0c0", mirror_id, "right"),
    ] {
        let mut new_part = document
            .parts
            .iter()
            .find(|part| part.id == old_id)
            .unwrap()
            .clone();
        new_part.id = new_id.into();
        document.parts.push(new_part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .part_ids
            .push(new_id.into());
    }
    document.matrices.extend([source_matrix, mirror_matrix]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend([
            serde_json::from_value(corr("matrix", "left-legacy", "right-legacy")).unwrap(),
            serde_json::from_value(corr("part", source_id, mirror_id)).unwrap(),
        ]);

    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"},
        "change": {"add-feature": {"feature": {"kind": "polygon",
            "id": "right/legacy-anchor", "anchorPartId": mirror_id,
            "points": [{"x": 4, "y": 6}, {"x": 8, "y": 10}, {"x": 12, "y": 14}],
            "operation": "add"}}},
        "seed": 18
    }});
    let translated = canonical(&document, &request).expect("the linked anchor translates");
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_eq!(
        value["edit"]["change"]["add-feature"]["feature"]["points"],
        json!([{"x": -12.0, "y": 14.0}, {"x": -8.0, "y": 10.0}, {"x": -4.0, "y": 6.0}])
    );
}

#[test]
fn local_outline_versions_stay_local_while_omitted_versions_refuse_edits() {
    let mut document = fixture();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend([
            serde_json::from_value(corr("outline-version", "left-local", "right-local")).unwrap(),
            serde_json::from_value(corr("outline-version", "left-omitted", "right-omitted"))
                .unwrap(),
        ]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .extend([
            serde_json::from_value(
                json!({"kind": "outline-version", "mirrorId": "right-local", "state": "local"}),
            )
            .unwrap(),
            serde_json::from_value(
                json!({"kind": "outline-version", "mirrorId": "right-omitted", "state": "omitted"}),
            )
            .unwrap(),
        ]);
    let local = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "version", "boardId": "right", "versionId": "right-local"},
        "change": "activate", "seed": 2
    }});
    expect_operation(&document, local.clone(), local);
    let omitted = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "version", "boardId": "right", "versionId": "right-omitted"},
        "change": "activate", "seed": 2
    }});
    assert!(refusal(&document, omitted).contains("omitted"));
}

#[test]
fn a_forward_projected_reviewed_contour_returns_exactly_to_its_source_shape() {
    let document = rotated_fixture();
    let source = vec![
        Vec2 {
            x: 1.23456789,
            y: -3.45678901,
        },
        Vec2 {
            x: 7.65432109,
            y: 8.76543210,
        },
        Vec2 {
            x: -9.87654321,
            y: 2.34567891,
        },
    ];
    let mirror: Vec<_> = source
        .iter()
        .rev()
        .map(|point| crate::mechanical::pair::forward(ROTATED, *point))
        .collect();
    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"},
        "change": {"edit-generated-perimeter": {"contour": 0, "points": mirror}},
        "seed": 9,
        "reviewed": [{"points": mirror, "hole": false}]
    }});
    let translated = canonical(&document, &request).expect("the reviewed contour translates");
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_close(
        &value["edit"]["reviewed"][0]["points"],
        &serde_json::to_value(source).unwrap(),
        "reviewed round trip",
    );
}

#[test]
fn current_projected_review_keeps_exact_source_coordinates_while_stale_review_stays_stale() {
    let mut document = rotated_fixture();
    document.boards[0].outline_ids.push("left/boundary".into());
    document.boards[1].outline_ids.push("right/boundary".into());
    let source_points = [
        Vec2 {
            x: 1.23456789,
            y: -3.45678901,
        },
        Vec2 {
            x: 7.65432109,
            y: 8.76543210,
        },
        Vec2 {
            x: -9.87654321,
            y: 2.34567891,
        },
    ];
    let mirror_points: Vec<_> = source_points
        .iter()
        .rev()
        .map(|point| crate::mechanical::pair::forward(ROTATED, *point))
        .collect();
    document.outline.extend([
        serde_json::from_value(json!({"kind": "polygon", "id": "left/boundary",
            "points": source_points, "operation": "add"}))
        .unwrap(),
        serde_json::from_value(json!({"kind": "polygon", "id": "right/boundary",
            "points": mirror_points, "operation": "add"}))
        .unwrap(),
    ]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(serde_json::from_value(corr("outline", "left/boundary", "right/boundary")).unwrap());
    let (cache, _, _) = crate::geometry::outlines(&document, None, &[]);
    let source_review =
        crate::geometry::generated_source(&document, &cache, &document.boards[0]).source;
    let mirror_review =
        crate::geometry::generated_source(&document, &cache, &document.boards[1]).source;
    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"}, "change": {"add-feature": {"feature": {
            "kind": "polygon", "id": "right/added", "points": [
                {"x": 8, "y": 9}, {"x": 10, "y": 12}, {"x": 14, "y": 9}], "operation": "add"}}},
        "seed": 11, "reviewed": mirror_review
    }});
    let translated = canonical(&document, &request).unwrap();
    let EditOperation::ApplyOutlineEdit { edit } = translated.operation else {
        panic!("the operation remains a typed outline edit")
    };
    assert_eq!(
        edit.reviewed.as_ref().unwrap(),
        &source_review,
        "inverse mapping preserves the exact current source review"
    );
    assert_eq!(
        crate::outline_edit::evaluate(&document, &edit).unwrap(),
        crate::outline_edit::OutlineEditDisposition::Changed
    );

    let mut relevant_change = document.clone();
    let Some(crate::model::OutlineFeature::Polygon { points, .. }) = relevant_change
        .outline
        .iter_mut()
        .find(|feature| feature.id() == "left/boundary")
    else {
        panic!("the reviewed source feature remains present")
    };
    points[0].x += 1.0;
    assert_eq!(
        crate::outline_edit::evaluate(&relevant_change, &edit).unwrap(),
        crate::outline_edit::OutlineEditDisposition::Retire,
        "a changed reviewed source retires the edit"
    );

    let mut unrelated_change = document.clone();
    unrelated_change.boards[1].name.push_str(" (renamed)");
    assert_eq!(
        crate::outline_edit::evaluate(&unrelated_change, &edit).unwrap(),
        crate::outline_edit::OutlineEditDisposition::Changed,
        "an unrelated board rename preserves the review"
    );

    let mut stale = request;
    stale["edit"]["reviewed"][0]["points"][0]["x"] = json!(12345.0);
    let translated = canonical(&document, &stale).unwrap();
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_ne!(
        value["edit"]["reviewed"],
        serde_json::to_value(source_review).unwrap()
    );
}

#[test]
fn apply_outline_edit_translates_fixed_and_linked_version_reviews() {
    let mut document = rotated_fixture();
    let mut versions = Vec::new();
    let mut source_versions = Vec::new();
    let mut mirror_versions = Vec::new();
    for (ownership, source_points) in [
        (
            "fixed",
            [
                Vec2 { x: 1.25, y: -3.5 },
                Vec2 { x: 7.75, y: 8.5 },
                Vec2 { x: -9.5, y: 2.25 },
            ],
        ),
        (
            "linked",
            [
                Vec2 { x: 21.5, y: 3.25 },
                Vec2 { x: 29.75, y: 9.0 },
                Vec2 { x: 16.25, y: 12.5 },
            ],
        ),
    ] {
        let mirror_points: Vec<_> = source_points
            .iter()
            .rev()
            .map(|point| crate::mechanical::pair::forward(ROTATED, *point))
            .collect();
        let source_id = format!("left-version-{ownership}");
        let mirror_id = format!("right-version-{ownership}");
        let source_feature_id = format!("left/{ownership}-outline");
        let mirror_feature_id = format!("right/{ownership}-outline");
        document.boards[0]
            .outline_ids
            .push(source_feature_id.clone());
        document.boards[1]
            .outline_ids
            .push(mirror_feature_id.clone());
        let source_feature = json!({"kind": "polygon", "id": source_feature_id,
            "points": source_points, "operation": "add"});
        let mirror_feature = json!({"kind": "polygon", "id": mirror_feature_id,
            "points": mirror_points, "operation": "add"});
        document.outline.extend(
            [source_feature.clone(), mirror_feature.clone()]
                .map(|feature| serde_json::from_value(feature).unwrap()),
        );
        versions.push((source_id.clone(), mirror_id.clone()));
        source_versions.push(json!({"id": source_id, "name": ownership,
            "ownership": ownership, "source": {"revision": 0},
            "geometry": {"features": [source_feature], "settings": {}, "expectedRegions": 1}}));
        mirror_versions.push(json!({"id": mirror_id, "name": ownership,
            "ownership": ownership, "source": {"revision": 0},
            "geometry": {"features": [mirror_feature], "settings": {}, "expectedRegions": 1}}));
    }
    document.board_outlines = vec![
        serde_json::from_value(
            json!({"boardId": "left", "activeVersionId": "left-version-fixed",
            "versions": source_versions}),
        )
        .unwrap(),
        serde_json::from_value(
            json!({"boardId": "right", "activeVersionId": "right-version-fixed",
            "versions": mirror_versions}),
        )
        .unwrap(),
    ];
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend(versions.iter().map(|(source, mirror)| {
            serde_json::from_value(corr("outline-version", source, mirror)).unwrap()
        }));
    let (cache, _, _) = crate::geometry::outlines(&document, None, &[]);

    for (ownership, (source_version, mirror_version)) in
        ["fixed", "linked"].into_iter().zip(versions)
    {
        let source_geometry = &document
            .board_outlines
            .iter()
            .find(|state| state.board_id == "left")
            .unwrap()
            .versions
            .iter()
            .find(|version| version.id == source_version)
            .unwrap()
            .geometry;
        let mirror_geometry = &document
            .board_outlines
            .iter()
            .find(|state| state.board_id == "right")
            .unwrap()
            .versions
            .iter()
            .find(|version| version.id == mirror_version)
            .unwrap()
            .geometry;
        let source_review =
            crate::geometry::feature_source_contours(&document, &cache, &source_geometry.features);
        let mirror_review =
            crate::geometry::feature_source_contours(&document, &cache, &mirror_geometry.features);
        let request = json!({"kind": "apply-outline-edit", "edit": {
            "target": {"kind": "version", "boardId": "right", "versionId": mirror_version},
            "change": {"set-margin": {"margin": 3.0}}, "seed": 5,
            "reviewed": mirror_review
        }});
        let translated = canonical(&document, &request)
            .unwrap_or_else(|error| panic!("the {ownership} version edit translates: {error}"));
        let value = serde_json::to_value(translated.operation).unwrap();
        assert_eq!(value["edit"]["target"]["boardId"], "left");
        assert_eq!(value["edit"]["target"]["versionId"], source_version);
        assert_close(
            &value["edit"]["reviewed"],
            &serde_json::to_value(&source_review).unwrap(),
            &format!("{ownership} version review"),
        );

        let mut stale = request;
        stale["edit"]["reviewed"][0]["points"][0]["x"] = json!(54321.0);
        let translated = canonical(&document, &stale).unwrap();
        let value = serde_json::to_value(translated.operation).unwrap();
        assert_ne!(
            value["edit"]["reviewed"],
            serde_json::to_value(&source_review).unwrap()
        );
    }
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
    assert_eq!(
        result.additions,
        vec![PartAddition {
            source_id: "left/LED".into(),
            mirror_id: "right/LED".into(),
            mirror_definition_id: "u".into(),
            mirror_reference: "right-D9".into(),
            mirror_layout_id: None,
        }]
    );
}

#[test]
fn a_controller_added_on_the_mirror_lands_on_the_source_without_the_half_turn() {
    // A controller definition in the document switches the AddPart inverse to the reflected
    // heading. The axis map sends (x, y) to (200 - x, y): the authored mirror pose (230, 12)
    // at heading 30 lands on the source (-30, 12) at heading -30 = 330, not 150.
    let mut document = fixture();
    document.definitions.push(
        serde_json::from_value(json!({
            "id": "mcu", "name": "MCU", "kind": "controller",
            "courtyard": [{"x":-5,"y":-4},{"x":5,"y":-4},{"x":5,"y":4},{"x":-5,"y":4}],
            "pads": []
        }))
        .unwrap(),
    );
    let request = json!({"kind": "add-part", "boardId": "right", "part": {
        "id": "right/MCU", "definitionId": "mcu", "reference": "right-MCU",
        "pose": {"at": {"x": 230, "y": 12}, "rotation": 30}, "side": "front"}});
    let result = canonical(&document, &request).unwrap();
    let EditOperation::AddPart { part, .. } = result.operation else {
        panic!("an addition is expected");
    };
    assert_close(
        &serde_json::to_value(part.pose).unwrap(),
        &json!({"at": {"x": -30, "y": 12}, "rotation": 330}),
        "pose",
    );
    assert_eq!(result.additions[0].source_id, "left/MCU");
    assert_eq!(result.additions[0].mirror_id, "right/MCU");
    // The controller rule is its own inverse: the source addition reflects back to exactly
    // what was authored on the mirror, (200 - -30, 12) at (0 - 330) mod 360 = 30.
    assert_eq!(200.0 - part.pose.at.x, 230.0);
    assert_eq!(part.pose.at.y, 12.0);
    assert_eq!((0.0 - part.pose.rotation).rem_euclid(360.0), 30.0);
}

#[test]
fn a_rotated_pair_map_places_and_turns_an_added_part_back_on_the_source() {
    let request = json!({"kind": "add-part", "boardId": "right", "part": {
        "id": "right/LED", "definitionId": "u", "reference": "right-D9",
        "pose": {"at": {"x": 90, "y": -50}, "rotation": 30}, "side": "front"}});
    let result = canonical(&rotated_fixture(), &request).unwrap();
    let EditOperation::AddPart { part, .. } = result.operation else {
        panic!("an addition is expected");
    };
    // Mirror (90, -50) is the image of source (20, 10); the heading is 90 - 30 + 180.
    assert_close(
        &serde_json::to_value(part.pose).unwrap(),
        &json!({"at": {"x": 20, "y": 10}, "rotation": 240}),
        "pose",
    );
}

#[test]
fn a_source_side_addition_plans_its_mirror_without_changing_the_authored_request() {
    let source = json!({"kind": "add-part", "boardId": "left", "part": {
        "id": "left/LED", "definitionId": "u", "reference": "D9",
        "pose": {"at": {"x": 1, "y": 2}, "rotation": 30}, "side": "front"}});
    let result = canonical(&fixture(), &source).unwrap();
    let operation = serde_json::to_value(result.operation).unwrap();
    assert_close(&operation, &source, "operation");
    assert_eq!(result.additions.len(), 1);
    assert_eq!(result.additions[0].source_id, "left/LED");
    assert_eq!(result.additions[0].mirror_id, "right/LED");
}

#[test]
fn a_mirror_side_addition_maps_a_linked_layout_to_the_source_layout() {
    let request = json!({"kind": "add-part", "boardId": "right", "layoutId": "right-layout", "part": {
        "id": "right/LED", "definitionId": "u", "reference": "right-D9",
        "pose": {"at": {"x": 230, "y": 12}, "rotation": 180}, "side": "front"}});
    let result = canonical(&native_layout_fixture(), &request).unwrap();
    let EditOperation::AddPart {
        part,
        board_id,
        layout_id,
        ..
    } = result.operation
    else {
        panic!("the source-side addition remains AddPart")
    };
    assert_eq!(board_id.as_deref(), Some("left"));
    assert_eq!(part.id, "left/LED");
    assert_eq!(layout_id.as_deref(), Some("left-layout"));
    assert_eq!(result.additions[0].mirror_layout_id, None);
}

#[test]
fn a_mirror_side_addition_keeps_an_unpaired_target_layout_membership() {
    let mut document = native_layout_fixture();
    let mut local_layout = document.layouts[1].clone();
    local_layout.id = "right-local-layout".into();
    local_layout.name = "Target only".into();
    local_layout.part_ids.clear();
    document.layouts.push(local_layout);
    let request = json!({"kind": "add-part", "boardId": "right", "layoutId": "right-local-layout", "part": {
        "id": "right/LED", "definitionId": "u", "reference": "right-D9",
        "pose": {"at": {"x": 230, "y": 12}, "rotation": 180}, "side": "front"}});
    let result = canonical(&document, &request).unwrap();
    let EditOperation::AddPart { layout_id, .. } = result.operation else {
        panic!("the source-side addition remains AddPart")
    };
    assert!(layout_id.is_none());
    assert_eq!(
        result.additions[0].mirror_layout_id.as_deref(),
        Some("right-local-layout")
    );
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
    let EditOperation::AddPart { part, board_id, .. } = result.operation else {
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

#[test]
fn a_mirror_matrix_payload_becomes_the_source_matrix_through_the_inverse_pair_map() {
    let matrix = translated_matrix(
        &fixture(),
        mirror_payload(json!({
        "origin": {"x": 190, "y": 5}, "rotation": 10,
        "cells": [
            {"row": 0, "column": 0, "enabled": true, "offset": {"x": 1, "y": 2}, "rotation": 5,
             "assemblies": [
                {"id": "diode", "definitionId": "d", "offset": {"x": -8, "y": -3}, "rotation": -80}]},
            {"row": 0, "column": 1, "enabled": false}
        ]})),
    );
    // Source identity, name, board and members are the source's own; geometry is inverted.
    assert_eq!(matrix["id"], "left-keys");
    assert_eq!(matrix["name"], "Left keys");
    assert_eq!(matrix["boardId"], "left");
    assert_eq!(
        matrix["partIds"],
        json!([
            "matrix/left-keys/r0c0",
            "matrix/left-keys/r0c0/diode",
            "matrix/left-keys/r0c1"
        ])
    );
    assert_close(
        &matrix,
        &json!({
        "id": "left-keys", "name": "Left keys", "rows": 1, "columns": 2,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 10, "y": 5}, "definitionId": "sw",
        "boardId": "left", "rotation": -10,
        "partIds": [
            "matrix/left-keys/r0c0", "matrix/left-keys/r0c0/diode", "matrix/left-keys/r0c1"],
        "cells": [
            {"row": 0, "column": 0, "enabled": true, "offset": {"x": 1, "y": 2}, "rotation": 5,
             "assemblies": [
                {"id": "diode", "definitionId": "d", "offset": {"x": 8, "y": -3}, "rotation": 80}]},
            {"row": 0, "column": 1, "enabled": false}
        ]}),
        "matrix",
    );
}

#[test]
fn a_rotated_pair_map_turns_the_matrix_origin_and_heading_back_once() {
    let matrix = translated_matrix(
        &rotated_fixture(),
        mirror_payload(json!({"origin": {"x": 90, "y": -50}})),
    );
    // Mirror origin (90, -50) is the image of source (20, 10); no stored heading becomes the
    // map's 90 degrees on the source, and the mirror flag clears.
    assert_close(&matrix["origin"], &json!({"x": 20, "y": 10}), "origin");
    assert_close(&matrix["rotation"], &json!(90), "rotation");
    assert!(
        matrix.get("mirror").is_none(),
        "the source frame is not flipped"
    );
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

#[test]
fn a_literal_mirror_companion_flips_offset_and_turn() {
    let matrix = companion_matrix(&fixture(), "d", -60.0);
    assert_close(
        &matrix["cells"][0]["assemblies"],
        &json!([{"id": "diode", "definitionId": "d", "offset": {"x": 8, "y": -3}, "rotation": 60}]),
        "assemblies",
    );
}

#[test]
fn a_tweaked_companion_subtracts_its_tweak_before_flipping() {
    let mut document = fixture();
    set_companion(
        &mut document,
        "literal-mirror-with-tweak",
        json!({"x": 1.0, "y": 2.0, "rotation": 10.0}),
    );
    // Payload offset (-8, -3) and turn -60: source x = -(-8 - 1), y = -3 - 2, turn = -(-60 - 10).
    let matrix = companion_matrix(&document, "d", -60.0);
    assert_close(
        &matrix["cells"][0]["assemblies"],
        &json!([{"id": "diode", "definitionId": "d", "offset": {"x": 9, "y": -5}, "rotation": 70}]),
        "assemblies",
    );
}

#[test]
fn a_duplicate_position_companion_keeps_the_mirror_values_as_the_source_values() {
    let mut document = fixture();
    set_companion(&mut document, "duplicate-position", json!(null));
    let matrix = companion_matrix(&document, "d", -60.0);
    assert_close(
        &matrix["cells"][0]["assemblies"],
        &json!([{"id": "diode", "definitionId": "d", "offset": {"x": -8, "y": -3}, "rotation": -60}]),
        "assemblies",
    );
}

#[test]
fn a_stabiliser_recovers_its_source_turn_from_its_keys_world_heading() {
    let matrix = companion_matrix(&fixture(), "kicad:marbastlib/STAB_MX_2u", -60.0);
    // Cell 0: stored Key rotation copies unchanged; the source heading is +20, so -60 - 2(20) = -100.
    assert_close(
        &matrix["cells"][0]["assemblies"][0],
        &json!({"id": "diode", "definitionId": "kicad:marbastlib/STAB_MX_2u",
                "offset": {"x": 8, "y": -3}, "rotation": -100}),
        "cell 0",
    );
    // Cell 1: key heading is the column splay 15; the turn is 10 + 0 - 30 = -20.
    assert_close(
        &matrix["cells"][1]["assemblies"][0],
        &json!({"id": "diode", "definitionId": "kicad:marbastlib/STAB_MX_2u",
                "offset": {"x": 8, "y": -3}, "rotation": -20}),
        "cell 1",
    );
}

#[test]
fn a_mirror_only_matrix_value_or_unrepresentable_cell_is_refused_not_dropped() {
    for (edits, needle) in [
        (json!({"name": "Renamed right"}), "name"),
        (json!({"definitionId": "other"}), "definition"),
        (json!({"boardId": "left"}), "board"),
        (json!({"mirror": "y"}), "horizontal"),
        (
            json!({"cells": [{"row": 0, "column": 0, "enabled": true, "assembliesLocal": true,
                "assemblies": []}]}),
            "local",
        ),
    ] {
        let message = refusal(&fixture(), set_matrix(mirror_payload(edits.clone())));
        assert!(message.contains(needle), "{edits}: {message}");
    }
}

#[test]
fn a_source_and_a_local_mirror_matrix_execute_as_written() {
    let source = set_matrix(json!({
        "id": "left-keys", "name": "Left keys", "rows": 1, "columns": 1,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 3, "y": 3}, "definitionId": "sw",
        "boardId": "left", "partIds": []}));
    expect_operation(&fixture(), source.clone(), source);
    let local = set_matrix(mirror_payload(json!({"name": "Local rename"})));
    expect_operation(&with_matrix_state(fixture(), "local"), local.clone(), local);
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

#[test]
fn a_matrix_added_on_the_mirror_board_becomes_a_source_matrix_with_a_planned_pair() {
    let result = canonical(
        &fixture(),
        &new_matrix("right-thumbs", "right", (190.0, 40.0)),
    )
    .unwrap();
    let value = serde_json::to_value(&result.operation).unwrap();
    assert_close(
        &value["matrix"],
        &json!({
            "id": "left-thumbs", "name": "Thumbs", "rows": 1, "columns": 2,
            "pitch": {"x": 19, "y": 19}, "origin": {"x": 10, "y": 40}, "definitionId": "sw",
            "boardId": "left", "partIds": [],
            "cells": [{"row": 0, "column": 0, "enabled": true, "assemblies": [
                {"id": "diode", "definitionId": "d", "offset": {"x": 8, "y": -2},
                 "rotation": 90}]}]}),
        "matrix",
    );
    let planned = addition(&result);
    assert_eq!(planned.source_id, "left-thumbs");
    assert_eq!(planned.mirror_id, "right-thumbs");
    assert_eq!(planned.source_board_id, "left");
    assert_eq!(planned.mirror_board_id, "right");
    // The cell with no stored entry is an enabled default cell, so it is a member too.
    let pair = |source: &str, mirror: &str| (source.to_owned(), mirror.to_owned());
    assert_eq!(
        planned.members,
        vec![
            pair("matrix/left-thumbs/r0c0", "matrix/right-thumbs/r0c0"),
            pair(
                "matrix/left-thumbs/r0c0/diode",
                "matrix/right-thumbs/r0c0/diode"
            ),
            pair("matrix/left-thumbs/r0c1", "matrix/right-thumbs/r0c1"),
        ]
    );
}

#[test]
fn a_matrix_added_on_the_source_board_runs_as_written_and_plans_its_mirror() {
    let request = new_matrix("left-thumbs", "left", (10.0, 40.0));
    let result = canonical(&fixture(), &request).unwrap();
    assert_close(
        &serde_json::to_value(&result.operation).unwrap(),
        &request,
        "operation",
    );
    let planned = addition(&result);
    assert_eq!(planned.source_id, "left-thumbs");
    assert_eq!(planned.mirror_id, "right-thumbs");
    assert_eq!(planned.members.len(), 3);
    assert_eq!(planned.members[1].1, "matrix/right-thumbs/r0c0/diode");
}

#[test]
fn a_planned_matrix_identity_skips_ids_already_in_use() {
    let mut document = fixture();
    document.matrices.push(
        serde_json::from_value(json!({"id": "left-thumbs", "rows": 1, "columns": 1,
            "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": 90}, "definitionId": "sw",
            "boardId": "left", "partIds": []}))
        .unwrap(),
    );
    let result = canonical(
        &document,
        &new_matrix("right-thumbs", "right", (190.0, 40.0)),
    )
    .unwrap();
    assert_eq!(addition(&result).source_id, "left-thumbs-2");
    assert_eq!(addition(&result).mirror_id, "right-thumbs");
    let value = serde_json::to_value(&result.operation).unwrap();
    assert_eq!(value["matrix"]["id"], "left-thumbs-2");
}

#[test]
fn an_existing_unpaired_matrix_and_a_matrix_on_an_unpaired_board_are_not_additions() {
    let mut document = fixture();
    document.matrices.push(
        serde_json::from_value(json!({"id": "right-extra", "rows": 1, "columns": 1,
            "pitch": {"x": 19, "y": 19}, "origin": {"x": 300, "y": 0}, "definitionId": "sw",
            "boardId": "right", "partIds": []}))
        .unwrap(),
    );
    let existing = set_matrix(json!({"id": "right-extra", "rows": 1, "columns": 1,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 310, "y": 0}, "definitionId": "sw",
        "boardId": "right", "partIds": []}));
    let result = canonical(&document, &existing).unwrap();
    assert!(result.matrix_additions.is_empty());
    expect_operation(&document, existing.clone(), existing);
    let elsewhere = new_matrix("other-thumbs", "other-board", (0.0, 0.0));
    let result = canonical(&fixture(), &elsewhere).unwrap();
    assert!(result.matrix_additions.is_empty());
    expect_operation(&fixture(), elsewhere.clone(), elsewhere);
}

#[test]
fn matrix_definitions_travel_with_the_translated_request_unchanged() {
    let definition =
        json!({"id": "extra", "name": "Extra", "kind": "passive", "courtyard": [], "pads": []});
    let result = canonical(
        &fixture(),
        &json!({"kind": "set-matrix", "matrix": mirror_payload(json!({})),
                "definitions": [definition]}),
    )
    .unwrap();
    let value = serde_json::to_value(&result.operation).unwrap();
    assert_eq!(value["definitions"], json!([definition]));
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

// ---- round trips through the production creation path --------------------------------------
//
// `CreateMirroredHalf` is the production forward derivation. Whatever it stores for the mirror
// must translate back to what it stored for the source, for every placement mode and for maps
// with a translation along the mirror line and a slanted axis.

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
    crate::matrix::set_matrix(&mut document, &matrix).expect("the source matrix materialises");
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
    crate::apply(&mut document, &[], &op(request)).expect("the half is created");
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
    let result = canonicalize(document, &request).expect("the mirror matrix translates");
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
                    positions: vec![crate::model::Position {
                        id: mirror.id.clone(),
                        at: mirror.pose.at,
                    }],
                };
                let result = canonicalize(&document, &request).unwrap();
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
