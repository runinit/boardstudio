//! Framed-constraint tests that need the real `CoreEngine` or the mirrored-half placement
//! authority.
//!
//! They live here, one layer above `boardstudio-core-kernel`, so that crate's own tests do not
//! depend on the engine (a dev-dependency cycle would hold its test binary back until the whole
//! engine graph is built). The fixtures repeat the kernel tests' `framed_tests` documents: pair
//! `pair/A` maps the alpha instance to the beta instance with theta = 36.86989764584402 and
//! t = (20, -7).

use boardstudio_core_engine::CoreEngine;
use boardstudio_core_engine::model::{
    Constraint, CoreReply, CoreRequest, MirroredCompanionMode as Mode, MirroredCompanionTweak,
    Pose2, ProjectDoc, Vec2,
};
use boardstudio_core_mirrored_half::mirrored_half;
use boardstudio_model::constraints::endpoints::{ConstraintEndpoint, Side, resolve_endpoint};
use boardstudio_model::constraints::follow_pose;
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

fn linked(source: &str, mirror: &str) -> ConstraintEndpoint {
    ConstraintEndpoint::LinkedMirror {
        pair_id: "pair/A".into(),
        source_part_id: source.into(),
        mirror_part_id: mirror.into(),
    }
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
fn a_native_legacy_offset_to_a_linked_mirror_survives_open_reconciliation() {
    let mut doc = document((20.0, -7.0));
    doc.definitions = serde_json::from_value(json!([{
        "id": "key", "name": "Key", "kind": "switch", "courtyard": [], "pads": []
    }]))
    .unwrap();
    doc.boards = serde_json::from_value(json!([
        {"id":"alpha", "name":"Alpha", "outlineIds":[], "partIds":["alpha/T","alpha/S"], "netIds":[], "thickness":1.6},
        {"id":"beta", "name":"Beta", "outlineIds":[], "partIds":["beta/T","beta/S"], "netIds":[], "thickness":1.6}
    ]))
    .unwrap();
    doc.constraints.push(legacy_offset(
        "c/legacy-linked",
        "alpha/S",
        "beta/T",
        5.0,
        -2.0,
    ));

    let mut engine = CoreEngine::new();
    let reply = engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let CoreReply::Scene {
        document: opened, ..
    } = reply
    else {
        panic!(
            "the legacy relation compiles through ownership and survives mirror reconcile: {reply:?}"
        );
    };

    let source = pose_of(&opened, "alpha/S");
    let target = pose_of(&opened, "beta/T");
    assert!((target.at.x - source.at.x - 5.0).abs() < TOLERANCE);
    assert!((target.at.y - source.at.y + 2.0).abs() < TOLERANCE);
    assert_eq!(target.rotation, source.rotation);
}

// ---- parity with the placement authority ---------------------------------------------------

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

fn world_pose_by_derive(
    doc: &ProjectDoc,
    mode: Mode,
    tweak: Option<MirroredCompanionTweak>,
    stabiliser: bool,
    fallback: bool,
) -> Pose2 {
    let part = |id: &str| doc.parts.iter().find(|part| part.id == id).unwrap();
    let (key, companion) = (part(KEY), part(COMPANION));
    let mapping = doc.hardware.as_ref().unwrap().pairs[0].mapping;
    // The source assembly in the key's frame, then the matrix reflection: x and heading flip.
    let local = boardstudio_core_kernel::matrix::splay::rotate(
        Vec2 {
            x: companion.pose.at.x - key.pose.at.x,
            y: companion.pose.at.y - key.pose.at.y,
        },
        -key.pose.rotation,
        Vec2::default(),
    );
    let (offset, rotation) = mirrored_half::companion_placement(
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
        at: boardstudio_model::mechanical::pair::forward(mapping, key.pose.at),
        rotation: mapping.rotation - key.pose.rotation,
    };
    follow_pose(
        mirrored_key,
        Pose2 {
            at: offset,
            rotation: rotation.unwrap(),
        },
    )
}

fn world_pose_by_endpoint(doc: &ProjectDoc) -> Pose2 {
    let endpoint =
        resolve_endpoint(doc, "c", Side::Source, &linked(COMPANION, MIRROR_COMPANION)).unwrap();
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
            Some(MirroredCompanionTweak {
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
