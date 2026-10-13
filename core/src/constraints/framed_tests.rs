//! Framed relations resolved through the shared solver graph, against hand-computed oracles.
//!
//! Pair P maps the alpha instance to the beta instance with theta = 36.86989764584402 and
//! t = (20, -7), so its independent position map is
//! `F(x, y) = (20 - 0.8x - 0.6y, -7 - 0.6x + 0.8y)`.

use super::endpoints::{ConstraintEndpoint, FramedConstraint, Side};
use super::relation::ConstraintRelation;
use super::resolve_prepared;
use crate::model::{Constraint, Pose2, ProjectDoc, Vec2};
use serde_json::json;

const TOLERANCE: f64 = 1e-9;

fn part(id: &str, x: f64, y: f64, rotation: f64) -> serde_json::Value {
    json!({
        "id": id, "definitionId": "key", "reference": id, "side": "front",
        "pose": {"at": {"x": x, "y": y}, "rotation": rotation}
    })
}

/// alpha/T and alpha/S are authored; beta/T and beta/S mirror them through pair/A. local/P and
/// the free parts have no pair.
fn document(translation: (f64, f64)) -> ProjectDoc {
    let mut doc = ProjectDoc::empty("p", "P");
    doc.parts = serde_json::from_value(json!([
        part("alpha/T", 4.2, 14.4, 161.86989764584402),
        part("beta/T", 8.0, 2.0, 55.0),
        part("alpha/S", 2.0, 5.0, 25.0),
        part("beta/S", 15.4, -4.2, 191.86989764584402),
        part("local/P", 3.0, 4.0, 40.0),
        part("free/Q", 0.0, 0.0, 0.0),
        part("free/R", 1.0, 1.0, 0.0),
    ]))
    .unwrap();
    doc.hardware = Some(
        serde_json::from_value(json!({
            "instances": [
                {"id": "alpha-instance", "name": "A", "boardId": "alpha", "half": "left",
                 "role": "primary", "flipped": false, "controllerPartId": null, "mechanical": null},
                {"id": "beta-instance", "name": "B", "boardId": "beta", "half": "right",
                 "role": "secondary", "flipped": true, "controllerPartId": null, "mechanical": null}
            ],
            "pairs": [{
                "id": "pair/A",
                "members": ["alpha-instance", "beta-instance"],
                "mapping": {"at": {"x": translation.0, "y": translation.1},
                            "rotation": 36.86989764584402},
                "construction": {
                    "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "plateThickness": 1.5, "plateFoamThickness": 0.0, "bottomFoamThickness": 0.0,
                    "bottomThickness": 1.5, "plateToPcb": 5.0, "wallThickness": 2.0,
                    "clearance": 0.2
                },
                "constructionSource": "alpha-instance",
                "halves": {
                    "sourceBoardId": "alpha", "mirrorBoardId": "beta",
                    "correspondences": [
                        {"kind": "part", "sourceId": "alpha/T", "mirrorId": "beta/T"},
                        {"kind": "part", "sourceId": "alpha/S", "mirrorId": "beta/S"}
                    ]
                }
            }]
        }))
        .unwrap(),
    );
    doc
}

fn pose_of(doc: &ProjectDoc, id: &str) -> Pose2 {
    doc.parts.iter().find(|part| part.id == id).unwrap().pose
}

fn assert_pose(doc: &ProjectDoc, id: &str, x: f64, y: f64, rotation: f64) {
    let actual = pose_of(doc, id);
    assert!(
        (actual.at.x - x).abs() < TOLERANCE
            && (actual.at.y - y).abs() < TOLERANCE
            && (actual.rotation - rotation).abs() < TOLERANCE,
        "{id} is {actual:?}, expected ({x}, {y}) heading {rotation}"
    );
}

fn part_endpoint(id: &str) -> ConstraintEndpoint {
    ConstraintEndpoint::Part { part_id: id.into() }
}

fn linked(source: &str, mirror: &str) -> ConstraintEndpoint {
    ConstraintEndpoint::LinkedMirror {
        pair_id: "pair/A".into(),
        source_part_id: source.into(),
        mirror_part_id: mirror.into(),
    }
}

#[test]
fn compiling_legacy_mirror_endpoints_retains_authored_views_and_literal_offset() {
    let doc = document((20.0, -7.0));
    let legacy = Constraint::Offset {
        id: "durable/constraint-7".into(),
        source_part_id: "beta/S".into(),
        target_part_id: "beta/T".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    };

    let compiled = super::endpoints::compile_legacy(&doc, &legacy).unwrap();
    let FramedConstraint::Framed {
        id,
        source,
        target,
        relation,
    } = compiled;

    assert_eq!(id, "durable/constraint-7");
    assert_eq!(source, linked("alpha/S", "beta/S"));
    assert_eq!(target, linked("alpha/T", "beta/T"));
    assert_eq!(
        relation,
        ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        }
    );
}

fn framed(
    id: &str,
    source: ConstraintEndpoint,
    target: ConstraintEndpoint,
    relation: ConstraintRelation,
) -> FramedConstraint {
    FramedConstraint::Framed {
        id: id.into(),
        source,
        target,
        relation,
    }
}

fn offset_5_neg2() -> ConstraintRelation {
    ConstraintRelation::Offset {
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    }
}

fn set_at(doc: &mut ProjectDoc, id: &str, x: f64, y: f64) {
    doc.parts
        .iter_mut()
        .find(|part| part.id == id)
        .unwrap()
        .pose
        .at = Vec2 { x, y };
}

fn set_heading(doc: &mut ProjectDoc, id: &str, rotation: f64) {
    doc.parts
        .iter_mut()
        .find(|part| part.id == id)
        .unwrap()
        .pose
        .rotation = rotation;
}

fn legacy_offset(id: &str, source: &str, target: &str, x: f64, y: f64) -> Constraint {
    Constraint::Offset {
        id: id.into(),
        source_part_id: source.into(),
        target_part_id: target.into(),
        offset: Vec2 { x, y },
        rotation: 0.0,
    }
}

#[test]
fn a_slanted_axis_reflects_an_ordinary_part_to_the_literal_oracle() {
    let mut doc = document((20.0, -7.0));
    let relation = ConstraintRelation::MirrorLine {
        at: Vec2 { x: 1.0, y: -2.0 },
        angle: 53.13010235415598,
    };
    let record = framed(
        "c/slant",
        part_endpoint("alpha/S"),
        part_endpoint("free/Q"),
        relation,
    );
    let changed = resolve_prepared(&mut doc, &[], &[record]).unwrap();
    assert_eq!(changed, vec!["free/Q"]);
    assert_pose(&doc, "free/Q", 7.44, 0.92, 81.26020470831196);
}

#[test]
fn a_local_source_drives_the_authority_of_a_linked_target_in_its_authored_direction() {
    let mut doc = document((20.0, -7.0));
    set_at(&mut doc, "alpha/T", 0.0, 0.0);
    let record = framed(
        "c/mixed",
        part_endpoint("local/P"),
        linked("alpha/T", "beta/T"),
        offset_5_neg2(),
    );
    resolve_prepared(&mut doc, &[], std::slice::from_ref(&record)).unwrap();
    // The visible target is (8, 2) heading 55; its authority is F^-1 of that.
    assert_pose(&doc, "alpha/T", 4.2, 14.4, 161.86989764584402);

    // One fixed canonical offset cannot hold: moving L moves T by a different vector.
    set_at(&mut doc, "local/P", 4.0, 6.0);
    resolve_prepared(&mut doc, &[], &[record]).unwrap();
    assert_pose(&doc, "alpha/T", 2.2, 15.4, 161.86989764584402);
}

#[test]
fn the_pair_translation_is_read_live_not_frozen_in_the_endpoint() {
    let mut doc = document((22.0, -8.0));
    let record = framed(
        "c/mixed",
        part_endpoint("local/P"),
        linked("alpha/T", "beta/T"),
        offset_5_neg2(),
    );
    resolve_prepared(&mut doc, &[], &[record]).unwrap();
    // The visible target stays at (8, 2); its authority moves with the translation.
    assert_pose(&doc, "alpha/T", 5.2, 16.4, 161.86989764584402);
}

#[test]
fn a_linked_source_drives_a_local_target() {
    let mut doc = document((20.0, -7.0));
    let record = framed(
        "c/out",
        linked("alpha/S", "beta/S"),
        part_endpoint("local/P"),
        offset_5_neg2(),
    );
    resolve_prepared(&mut doc, &[], std::slice::from_ref(&record)).unwrap();
    assert_pose(&doc, "local/P", 20.4, -6.2, 206.86989764584402);
    set_at(&mut doc, "alpha/S", 3.0, 7.0);
    resolve_prepared(&mut doc, &[], &[record]).unwrap();
    assert_pose(&doc, "local/P", 18.4, -5.2, 206.86989764584402);
}

#[test]
fn legacy_records_and_framed_records_share_one_graph() {
    let mut doc = document((20.0, -7.0));
    // free/R follows the framed result of free/Q through a legacy offset.
    doc.constraints
        .push(legacy_offset("legacy", "free/Q", "free/R", 1.0, 2.0));
    let record = framed(
        "c/mixed",
        part_endpoint("local/P"),
        part_endpoint("free/Q"),
        offset_5_neg2(),
    );
    resolve_prepared(&mut doc, &[], &[record]).unwrap();
    assert_pose(&doc, "free/Q", 8.0, 2.0, 55.0);
    assert_pose(&doc, "free/R", 9.0, 4.0, 55.0);
}

fn refuses(doc: &mut ProjectDoc, records: &[FramedConstraint], needle: &str) {
    let before = doc.clone();
    let message = resolve_prepared(doc, &[], records).unwrap_err();
    assert!(message.contains(needle), "{message:?} lacks {needle:?}");
    assert_eq!(*doc, before, "a refusal changed the document");
}

#[test]
fn bindings_that_do_not_match_a_live_correspondence_are_refused_atomically() {
    // Stale: beta/S is not the mirror of alpha/T.
    refuses(
        &mut document((20.0, -7.0)),
        &[framed(
            "c",
            part_endpoint("local/P"),
            linked("alpha/T", "beta/S"),
            offset_5_neg2(),
        )],
        "correspondence",
    );
    // Unknown pair.
    let mut unknown = linked("alpha/T", "beta/T");
    if let ConstraintEndpoint::LinkedMirror { pair_id, .. } = &mut unknown {
        *pair_id = "pair/none".into();
    }
    refuses(
        &mut document((20.0, -7.0)),
        &[framed(
            "c",
            part_endpoint("local/P"),
            unknown,
            offset_5_neg2(),
        )],
        "pair/none",
    );
    // Missing part.
    refuses(
        &mut document((20.0, -7.0)),
        &[framed(
            "c",
            part_endpoint("ghost"),
            part_endpoint("free/Q"),
            offset_5_neg2(),
        )],
        "missing",
    );
    // A Part endpoint cannot name a part that a linked mirror owns.
    refuses(
        &mut document((20.0, -7.0)),
        &[framed(
            "c",
            part_endpoint("local/P"),
            part_endpoint("beta/T"),
            offset_5_neg2(),
        )],
        "linked mirror",
    );
}

fn with_exception(state: &str) -> ProjectDoc {
    let mut doc = document((20.0, -7.0));
    doc.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions = serde_json::from_value(json!([
        {"kind": "part", "mirrorId": "beta/T", "state": state}
    ]))
    .unwrap();
    doc
}

#[test]
fn local_and_omitted_mirrors_cannot_be_addressed_as_linked() {
    for state in ["local", "omitted"] {
        refuses(
            &mut with_exception(state),
            &[framed(
                "c",
                part_endpoint("local/P"),
                linked("alpha/T", "beta/T"),
                offset_5_neg2(),
            )],
            state,
        );
    }
}

#[test]
fn a_local_mirror_is_an_ordinary_part_endpoint() {
    let mut doc = with_exception("local");
    let record = framed(
        "c",
        part_endpoint("local/P"),
        part_endpoint("beta/T"),
        offset_5_neg2(),
    );
    resolve_prepared(&mut doc, &[], &[record]).unwrap();
    assert_pose(&doc, "beta/T", 8.0, 2.0, 55.0);
}

#[test]
fn two_visible_targets_sharing_one_authority_are_refused() {
    let mut doc = document((20.0, -7.0));
    doc.constraints
        .push(legacy_offset("legacy", "free/Q", "alpha/T", 0.0, 0.0));
    refuses(
        &mut doc,
        &[framed(
            "c",
            part_endpoint("local/P"),
            linked("alpha/T", "beta/T"),
            offset_5_neg2(),
        )],
        "multiple constraints",
    );
}

#[test]
fn cycles_through_an_alias_are_refused() {
    // Source and target are different visible IDs of one authority.
    refuses(
        &mut document((20.0, -7.0)),
        &[framed(
            "c",
            linked("alpha/T", "beta/T"),
            part_endpoint("alpha/T"),
            offset_5_neg2(),
        )],
        "cycle",
    );
    // A two-node cycle through a framed and a legacy record.
    let mut doc = document((20.0, -7.0));
    doc.constraints
        .push(legacy_offset("legacy", "alpha/T", "free/Q", 0.0, 0.0));
    refuses(
        &mut doc,
        &[framed(
            "c",
            part_endpoint("free/Q"),
            linked("alpha/T", "beta/T"),
            offset_5_neg2(),
        )],
        "cycle",
    );
}

#[test]
fn non_finite_and_duplicate_records_are_refused() {
    let bad_axis = ConstraintRelation::MirrorLine {
        at: Vec2 {
            x: 0.0,
            y: f64::INFINITY,
        },
        angle: 10.0,
    };
    refuses(
        &mut document((20.0, -7.0)),
        &[framed(
            "c",
            part_endpoint("alpha/S"),
            part_endpoint("free/Q"),
            bad_axis,
        )],
        "non-finite",
    );
    refuses(
        &mut document((20.0, -7.0)),
        &[
            framed(
                "c",
                part_endpoint("alpha/S"),
                part_endpoint("free/Q"),
                offset_5_neg2(),
            ),
            framed(
                "c",
                part_endpoint("alpha/S"),
                part_endpoint("free/R"),
                offset_5_neg2(),
            ),
        ],
        "Duplicate or empty",
    );
}

#[test]
fn an_unrelated_part_is_left_alone() {
    let mut doc = document((20.0, -7.0));
    let before = pose_of(&doc, "free/R");
    let record = framed(
        "c",
        part_endpoint("local/P"),
        linked("alpha/T", "beta/T"),
        offset_5_neg2(),
    );
    resolve_prepared(&mut doc, &[], &[record]).unwrap();
    assert_eq!(pose_of(&doc, "free/R"), before);
}

/// Every stored kind, compiled into a framed relation between plain parts, places its target
/// where the stored solver does, within 1e-9: the exact (bit-for-bit) pin for axis-aligned
/// mirrors is `axis_aligned_mirrors_reproduce_the_stored_solver_bit_for_bit` in relation.rs.
#[test]
fn stored_records_and_their_compiled_relations_place_targets_within_tolerance() {
    use crate::model::{MirrorAxis, OffsetAxes};

    let stored = |kind: usize| match kind {
        0 => Constraint::offset(
            OffsetAxes::Board,
            "c".into(),
            "alpha/S".into(),
            "free/Q".into(),
            Vec2 { x: 10.0, y: 2.0 },
            5.0,
        ),
        1 => Constraint::offset(
            OffsetAxes::Part,
            "c".into(),
            "alpha/S".into(),
            "free/Q".into(),
            Vec2 { x: 10.0, y: 2.0 },
            5.0,
        ),
        2 => Constraint::Mirror {
            id: "c".into(),
            source_part_id: "alpha/S".into(),
            target_part_id: "free/Q".into(),
            axis: MirrorAxis::Vertical,
            coordinate: -3.5,
        },
        _ => Constraint::Mirror {
            id: "c".into(),
            source_part_id: "alpha/S".into(),
            target_part_id: "free/Q".into(),
            axis: MirrorAxis::Horizontal,
            coordinate: 4.25,
        },
    };
    for kind in 0..4 {
        for (x, y, heading) in [(2.0, 5.0, 25.0), (-7.5, 3.0, 200.0), (0.0, 0.0, -90.0)] {
            let mut by_store = document((20.0, -7.0));
            set_at(&mut by_store, "alpha/S", x, y);
            set_heading(&mut by_store, "alpha/S", heading);
            let mut by_relation = by_store.clone();

            by_store.constraints.push(stored(kind));
            super::resolve(&mut by_store).unwrap();

            let record = framed(
                "c",
                part_endpoint("alpha/S"),
                part_endpoint("free/Q"),
                ConstraintRelation::from_legacy(&stored(kind)),
            );
            resolve_prepared(&mut by_relation, &[], &[record]).unwrap();

            let expected = pose_of(&by_store, "free/Q");
            assert_pose(
                &by_relation,
                "free/Q",
                expected.at.x,
                expected.at.y,
                expected.rotation,
            );
        }
    }
}

/// Until the persisted arm and format 6 land together with 111, a framed record cannot be
/// stored: an older reader must refuse it rather than drop it.
#[test]
fn a_framed_record_is_not_a_storable_constraint_kind_yet() {
    let record = json!({
        "kind": "framed", "id": "c",
        "source": {"kind": "part", "partId": "a"},
        "target": {"kind": "part", "partId": "b"},
        "relation": {"kind": "mirror-line", "at": {"x": 0, "y": 0}, "angle": 10}
    });
    assert!(serde_json::from_value::<Constraint>(record).is_err());
}

#[test]
fn a_pair_whose_construction_source_is_its_second_member_uses_the_inverse_map() {
    let mut doc = document((20.0, -7.0));
    // beta is now the authored half: beta/T owns the geometry and alpha/T is its mirror.
    let pair = &mut doc.hardware.as_mut().unwrap().pairs[0];
    pair.construction_source = Some("beta-instance".into());
    let halves = pair.halves.as_mut().unwrap();
    halves.source_board_id = "beta".into();
    halves.mirror_board_id = "alpha".into();
    halves.correspondences = serde_json::from_value(json!([
        {"kind": "part", "sourceId": "beta/T", "mirrorId": "alpha/T"}
    ]))
    .unwrap();
    set_at(&mut doc, "beta/T", 0.0, 0.0);
    let record = framed(
        "c/inverse",
        part_endpoint("local/P"),
        linked("beta/T", "alpha/T"),
        offset_5_neg2(),
    );
    resolve_prepared(&mut doc, &[], &[record]).unwrap();
    // The visible target is (8, 2) heading 55 in the mirror frame; its authority is the
    // stored map applied forward: F(8, 2) = (12.4, -10.2).
    assert_pose(&doc, "beta/T", 12.4, -10.2, 161.86989764584402);
}

// ---- matrix keys and companions -------------------------------------------------------

const KEY: &str = "matrix/alpha-m/r0c0";
const COMPANION: &str = "matrix/alpha-m/r0c0/diode";
const MIRROR_KEY: &str = "matrix/beta-m/r0c0";
const MIRROR_COMPANION: &str = "matrix/beta-m/r0c0/diode";

/// Key K = (2, 5) heading 0 and its companion C = (4, 8) heading 30 on the alpha half, with
/// their mirrors on beta. `definition` is the companion's definition, `mode` the pair's
/// companion placement and `tweak` the item's adjustment (u, v, w).
fn matrix_document(definition: &str, mode: &str, tweak: Option<(f64, f64, f64)>) -> ProjectDoc {
    let mut doc = document((20.0, -7.0));
    doc.definitions = serde_json::from_value(json!([
        {"id": "key", "name": "Key", "kind": "switch", "courtyard": [], "pads": []},
        {"id": "diode", "name": "Diode", "kind": "passive", "courtyard": [], "pads": []},
        {"id": "kicad:marbastlib/STAB_MX_2u", "name": "Stabiliser", "kind": "passive",
         "courtyard": [], "pads": []}
    ]))
    .unwrap();
    doc.parts = serde_json::from_value(json!([
        part(KEY, 2.0, 5.0, 0.0),
        part(COMPANION, 4.0, 8.0, 30.0),
        part(MIRROR_KEY, 15.4, -4.2, 36.86989764584402),
        part(MIRROR_COMPANION, 12.0, -3.0, 6.86989764584402),
        part("local/P", 3.0, 4.0, 40.0),
        part("free/Q", 0.0, 0.0, 0.0),
    ]))
    .unwrap();
    doc.matrices = serde_json::from_value(json!([
        {"id": "alpha-m", "rows": 1, "columns": 1, "pitch": {"x": 19, "y": 19},
         "origin": {"x": 2, "y": 5}, "definitionId": "key", "partIds": [KEY, COMPANION],
         "cells": [{"row": 0, "column": 0, "enabled": true, "assemblies": [
             {"id": "diode", "definitionId": definition,
              "offset": {"x": 2.0, "y": 3.0}, "rotation": 30.0}]}]},
        {"id": "beta-m", "rows": 1, "columns": 1, "pitch": {"x": 19, "y": 19},
         "origin": {"x": 15.4, "y": -4.2}, "definitionId": "key", "partIds": [MIRROR_KEY]}
    ]))
    .unwrap();
    let halves = doc.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    let item = |source: &str, mirror: &str| {
        let mut value = json!({"kind": "part", "sourceId": source, "mirrorId": mirror});
        if let Some((u, v, w)) = tweak
            && source == COMPANION
        {
            value["companionTweak"] = json!({"x": u, "y": v, "rotation": w});
            value["companionMode"] = json!("literal-mirror-with-tweak");
        }
        value
    };
    halves.correspondences = serde_json::from_value(json!([
        item(KEY, MIRROR_KEY),
        item(COMPANION, MIRROR_COMPANION),
    ]))
    .unwrap();
    halves.companion_mode = serde_json::from_value(json!(mode)).unwrap();
    doc
}

#[test]
fn a_local_part_drives_a_linked_key_through_the_key_projection() {
    let mut doc = matrix_document("diode", "literal-mirror", None);
    let record = framed(
        "c/key",
        part_endpoint("local/P"),
        linked(KEY, MIRROR_KEY),
        offset_5_neg2(),
    );
    resolve_prepared(&mut doc, &[], &[record]).unwrap();
    // The visible key is (8, 2) heading 55: its authority is F^-1(8, 2) with heading
    // theta - 55, with no independent half turn.
    assert_pose(&doc, KEY, 4.2, 14.4, -18.13010235415598);
}

#[test]
fn a_linked_key_drives_a_local_part() {
    let mut doc = matrix_document("diode", "literal-mirror", None);
    set_heading(&mut doc, KEY, 25.0);
    let record = framed(
        "c/out",
        linked(KEY, MIRROR_KEY),
        part_endpoint("free/Q"),
        offset_5_neg2(),
    );
    resolve_prepared(&mut doc, &[], &[record]).unwrap();
    // The visible key is (15.4, -4.2) heading theta - 25 = 11.8699.
    assert_pose(&doc, "free/Q", 20.4, -6.2, 26.86989764584402);
}

#[test]
fn a_local_part_drives_a_linked_companion_by_each_placement_rule() {
    // (definition, pair mode, tweak, authority x, y, heading)
    let cases = [
        // Literal: authority is F^-1 of the visible pose, heading theta - 55.
        (
            "diode",
            "literal-mirror",
            None,
            4.2,
            14.4,
            -18.13010235415598,
        ),
        // Duplicate: key-local pose is kept, so the key (2, 5) heading 0 is the origin.
        (
            "diode",
            "duplicate-position",
            None,
            -0.2,
            14.4,
            18.13010235415598,
        ),
    ];
    for (definition, mode, tweak, x, y, heading) in cases {
        let mut doc = matrix_document(definition, mode, tweak);
        let record = framed(
            "c/companion",
            part_endpoint("local/P"),
            linked(COMPANION, MIRROR_COMPANION),
            offset_5_neg2(),
        );
        resolve_prepared(&mut doc, &[], &[record]).unwrap();
        // The explicit relation outranks the companion's implicit placement on its key.
        assert_pose(&doc, COMPANION, x, y, heading);
    }
}

#[test]
fn a_linked_companion_drives_a_local_part_under_tweak_and_stabiliser_rules() {
    // Tweaked literal (1, -2, 7): visible (14, -4) heading 13.8699, then offset (5, -2) / 15.
    let mut tweaked = matrix_document("diode", "literal-mirror", Some((1.0, -2.0, 7.0)));
    let record = framed(
        "c/out",
        linked(COMPANION, MIRROR_COMPANION),
        part_endpoint("free/Q"),
        offset_5_neg2(),
    );
    resolve_prepared(&mut tweaked, &[], std::slice::from_ref(&record)).unwrap();
    assert_pose(&tweaked, "free/Q", 19.0, -6.0, 28.86989764584402);

    // A stabiliser keeps its world heading 30: visible (12, -3) heading 30.
    let mut stabiliser = matrix_document("kicad:marbastlib/STAB_MX_2u", "duplicate-position", None);
    resolve_prepared(&mut stabiliser, &[], &[record]).unwrap();
    assert_pose(&stabiliser, "free/Q", 17.0, -5.0, 45.0);
}

#[test]
fn a_cycle_through_a_companion_and_its_key_is_refused_atomically() {
    // The key is driven from the companion, and the companion's projection reads the key.
    let mut doc = matrix_document("diode", "literal-mirror", None);
    let records = [
        framed(
            "c/companion",
            part_endpoint("local/P"),
            linked(COMPANION, MIRROR_COMPANION),
            offset_5_neg2(),
        ),
        framed(
            "c/key",
            part_endpoint(COMPANION),
            part_endpoint(KEY),
            offset_5_neg2(),
        ),
    ];
    refuses(&mut doc, &records, "cycle");
}

// ---- the shared authoritative-target lookup -------------------------------------------

#[test]
fn the_driving_record_is_found_through_either_half_of_a_linked_pair() {
    let mut doc = document((20.0, -7.0));
    doc.constraints
        .push(legacy_offset("c/legacy", "free/Q", "alpha/T", 1.0, 2.0));
    for asked in ["alpha/T", "beta/T"] {
        let found = super::driving_constraint(&doc, asked).unwrap();
        assert_eq!(found.map(Constraint::id), Some("c/legacy"), "{asked}");
    }
    assert_eq!(super::driving_constraint(&doc, "free/R").unwrap(), None);
    assert_eq!(super::driving_constraint(&doc, "local/P").unwrap(), None);
}

#[test]
fn a_local_mirror_is_its_own_authority_for_the_lookup() {
    let mut doc = with_exception("local");
    doc.constraints
        .push(legacy_offset("c/legacy", "free/Q", "alpha/T", 1.0, 2.0));
    assert_eq!(super::driving_constraint(&doc, "beta/T").unwrap(), None);
    assert!(
        super::driving_constraint(&doc, "alpha/T")
            .unwrap()
            .is_some()
    );
}

#[test]
fn two_records_on_one_authority_are_a_located_error_not_a_guess() {
    let mut doc = document((20.0, -7.0));
    doc.constraints
        .push(legacy_offset("c/one", "free/Q", "alpha/T", 1.0, 2.0));
    doc.constraints
        .push(legacy_offset("c/two", "free/R", "beta/T", 1.0, 2.0));
    let message = super::driving_constraint(&doc, "beta/T").unwrap_err();
    assert!(
        message.contains("alpha/T") && message.contains("multiple constraints"),
        "{message}"
    );
}

/// The lookup is the activation-time contract; the production solver still takes two stored
/// records that name a source and its visible mirror, as every existing document may hold.
#[test]
fn a_stored_pair_on_one_authority_is_a_lookup_error_the_solver_accepts() {
    let mut doc = document((20.0, -7.0));
    doc.constraints
        .push(legacy_offset("c/one", "free/Q", "alpha/T", 1.0, 2.0));
    doc.constraints
        .push(legacy_offset("c/two", "free/R", "beta/T", 1.0, 2.0));
    assert!(super::driving_constraint(&doc, "alpha/T").is_err());
    super::resolve(&mut doc).unwrap();
}

#[test]
fn a_stored_record_and_a_framed_one_on_one_authority_are_refused_by_the_solver() {
    // The stored record names the visible mirror; the framed one names the source: one part.
    let mut doc = document((20.0, -7.0));
    doc.constraints
        .push(legacy_offset("c/legacy", "free/Q", "beta/T", 0.0, 0.0));
    // beta/T is a derived mirror, so the lookup, not the literal id, decides the conflict.
    refuses(
        &mut doc,
        &[framed(
            "c/framed",
            part_endpoint("local/P"),
            linked("alpha/T", "beta/T"),
            offset_5_neg2(),
        )],
        "multiple constraints",
    );
}

// ---- parity with the placement authority, and refusals of unproduced companions --------

fn world_pose_by_derive(
    doc: &ProjectDoc,
    mode: crate::model::MirroredCompanionMode,
    tweak: Option<crate::model::MirroredCompanionTweak>,
    stabiliser: bool,
    fallback: bool,
) -> Pose2 {
    let part = |id: &str| doc.parts.iter().find(|part| part.id == id).unwrap();
    let (key, companion) = (part(KEY), part(COMPANION));
    let mapping = doc.hardware.as_ref().unwrap().pairs[0].mapping;
    // The source assembly in the key's frame, then the matrix reflection: x and heading flip.
    let local = crate::matrix::splay::rotate(
        Vec2 {
            x: companion.pose.at.x - key.pose.at.x,
            y: companion.pose.at.y - key.pose.at.y,
        },
        -key.pose.rotation,
        Vec2::default(),
    );
    let (offset, rotation) = crate::mirrored_half::companion_placement(
        Vec2 {
            x: -local.x,
            y: local.y,
        },
        Some(-(companion.pose.rotation - key.pose.rotation)),
        key,
        companion,
        mapping,
        mode,
        tweak,
        stabiliser,
        fallback,
    )
    .unwrap();
    let mirrored_key = Pose2 {
        at: crate::mechanical::pair::forward(mapping, key.pose.at),
        rotation: mapping.rotation - key.pose.rotation,
    };
    super::follow_pose(
        mirrored_key,
        Pose2 {
            at: offset,
            rotation: rotation.unwrap(),
        },
    )
}

fn world_pose_by_endpoint(doc: &ProjectDoc) -> Pose2 {
    let endpoint = super::endpoints::resolve_endpoint(
        doc,
        "c",
        Side::Source,
        &linked(COMPANION, MIRROR_COMPANION),
    )
    .unwrap();
    endpoint
        .frame
        .with_key(Some(pose_of(doc, KEY)))
        .project(pose_of(doc, COMPANION))
        .unwrap()
}

fn assert_same_world_pose(actual: Pose2, expected: Pose2, label: &str) {
    let turn = (actual.rotation - expected.rotation).rem_euclid(360.0);
    assert!(
        (actual.at.x - expected.at.x).abs() < 1e-9
            && (actual.at.y - expected.at.y).abs() < 1e-9
            && turn.min(360.0 - turn) < 1e-9,
        "{label}: endpoint {actual:?}, derive {expected:?}"
    );
}

/// The endpoint rules are a second statement of `derive::companion_assembly`. They agree with
/// it to 1e-9 mm and degrees: the two compose the same rotations in a different order, so they
/// are not bit-equal, and nothing here is accumulated.
#[test]
fn companion_endpoint_rules_match_derive_for_every_rule_and_key_heading() {
    use crate::model::{MirroredCompanionMode as Mode, MirroredCompanionTweak as Tweak};
    let cases = [
        (
            "diode",
            "literal-mirror",
            None,
            Mode::LiteralMirror,
            None,
            false,
        ),
        (
            "diode",
            "duplicate-position",
            None,
            Mode::DuplicatePosition,
            None,
            false,
        ),
        (
            "diode",
            "literal-mirror",
            Some((1.0, -2.0, 7.0)),
            Mode::LiteralMirrorWithTweak,
            Some(Tweak {
                x: 1.0,
                y: -2.0,
                rotation: 7.0,
            }),
            false,
        ),
        (
            "kicad:marbastlib/STAB_MX_2u",
            "duplicate-position",
            None,
            Mode::DuplicatePosition,
            None,
            true,
        ),
    ];
    for (definition, pair_mode, tweak_spec, mode, tweak, stabiliser) in cases {
        for key_heading in [0.0, 90.0, -37.5] {
            let mut doc = matrix_document(definition, pair_mode, tweak_spec);
            set_heading(&mut doc, KEY, key_heading);
            assert_same_world_pose(
                world_pose_by_endpoint(&doc),
                world_pose_by_derive(&doc, mode, tweak, stabiliser, false),
                &format!("{definition} {pair_mode} key heading {key_heading}"),
            );
        }
    }
}

/// A repaired companion carries a persisted literal-mirror correspondence over the pair's
/// duplicate default; derive's fallback path ignores the mode and keeps the literal rule.
#[test]
fn a_repaired_companion_places_like_derives_fallback_not_the_pair_default() {
    use crate::model::MirroredCompanionMode as Mode;
    let mut default_doc = matrix_document("diode", "duplicate-position", None);
    set_heading(&mut default_doc, KEY, 90.0);
    let mut repaired = default_doc.clone();
    repaired.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences[1]
        .companion_mode = Some(Mode::LiteralMirror);

    let fallback = world_pose_by_derive(&repaired, Mode::DuplicatePosition, None, false, true);
    assert_same_world_pose(world_pose_by_endpoint(&repaired), fallback, "repaired");
    // The pair default alone would have placed it by the duplicate rule, elsewhere.
    let duplicate = world_pose_by_endpoint(&default_doc);
    assert!((duplicate.at.x - fallback.at.x).abs() > 1.0);
}

#[test]
fn a_companion_derive_would_never_produce_is_refused_not_projected_as_a_part() {
    let record = || {
        framed(
            "c/companion",
            part_endpoint("local/P"),
            linked(COMPANION, MIRROR_COMPANION),
            offset_5_neg2(),
        )
    };
    let mut disabled = matrix_document("diode", "literal-mirror", None);
    disabled.matrices[0].cells[0].enabled = false;
    refuses(&mut disabled, &[record()], "disabled or deleted");

    let mut deleted = matrix_document("diode", "literal-mirror", None);
    deleted.matrices[0].cells[0].deleted = true;
    refuses(&mut deleted, &[record()], "disabled or deleted");

    // The companion's key part is gone.
    let mut keyless = matrix_document("diode", "literal-mirror", None);
    keyless.parts.retain(|part| part.id != KEY);
    refuses(&mut keyless, &[record()], "key");
}
