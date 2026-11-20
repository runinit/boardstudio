use boardstudio_model::constraints::endpoints::*;
use boardstudio_model::constraints::relation::ConstraintRelation;
use boardstudio_model::model::{Pose2, Vec2};
use serde_json::json;

mod common;
use common::{assert_close, pair_map};

const TOLERANCE: f64 = 1e-9;

fn visible_part_id(endpoint: &ConstraintEndpoint) -> &str {
    match endpoint {
        ConstraintEndpoint::Part { part_id } => part_id,
        ConstraintEndpoint::LinkedMirror { mirror_part_id, .. } => mirror_part_id,
    }
}

fn authoritative_part_id(endpoint: &ConstraintEndpoint) -> &str {
    match endpoint {
        ConstraintEndpoint::Part { part_id } => part_id,
        ConstraintEndpoint::LinkedMirror { source_part_id, .. } => source_part_id,
    }
}

#[test]
fn linked_mirror_address_keeps_visible_identity_separate_from_authority() {
    let endpoint = ConstraintEndpoint::LinkedMirror {
        pair_id: "pair/A".into(),
        source_part_id: "alpha/T".into(),
        mirror_part_id: "beta/T".into(),
    };

    assert_eq!(visible_part_id(&endpoint), "beta/T");
    assert_eq!(authoritative_part_id(&endpoint), "alpha/T");
    assert_eq!(
        serde_json::to_value(endpoint).unwrap(),
        json!({
            "kind": "linked-mirror",
            "pairId": "pair/A",
            "sourcePartId": "alpha/T",
            "mirrorPartId": "beta/T"
        })
    );
}

#[test]
fn part_endpoint_uses_one_identity_for_visible_and_authoritative_address() {
    let endpoint = ConstraintEndpoint::Part {
        part_id: "local/P".into(),
    };

    assert_eq!(visible_part_id(&endpoint), "local/P");
    assert_eq!(authoritative_part_id(&endpoint), "local/P");
    assert_eq!(
        serde_json::to_value(endpoint).unwrap(),
        json!({"kind": "part", "partId": "local/P"})
    );
}

#[test]
fn framed_record_retains_global_id_and_source_to_target_direction() {
    let framed = FramedConstraint::Framed {
        id: "constraint/mixed-7".into(),
        source: ConstraintEndpoint::Part {
            part_id: "local/P".into(),
        },
        target: ConstraintEndpoint::LinkedMirror {
            pair_id: "pair/A".into(),
            source_part_id: "alpha/T".into(),
            mirror_part_id: "beta/T".into(),
        },
        relation: ConstraintRelation::Offset {
            offset: boardstudio_model::model::Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        },
    };

    assert_eq!(
        serde_json::to_value(framed.clone()).unwrap(),
        json!({
            "kind": "framed",
            "id": "constraint/mixed-7",
            "source": {"kind": "part", "partId": "local/P"},
            "target": {
                "kind": "linked-mirror",
                "pairId": "pair/A",
                "sourcePartId": "alpha/T",
                "mirrorPartId": "beta/T"
            },
            "relation": {
                "kind": "offset",
                "offset": {"x": 5.0, "y": -2.0},
                "rotation": 15.0
            }
        })
    );
    let round_trip: FramedConstraint =
        serde_json::from_value(serde_json::to_value(framed.clone()).unwrap()).unwrap();
    assert_eq!(round_trip, framed);
}

#[test]
fn identity_source_and_live_linked_target_solve_in_the_authored_direction() {
    let mapping = Pose2 {
        at: Vec2 { x: 20.0, y: -7.0 },
        rotation: 36.86989764584402,
    };
    let source = Pose2 {
        at: Vec2 { x: 3.0, y: 4.0 },
        rotation: 40.0,
    };
    let relation = ConstraintRelation::Offset {
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    };

    let target_authority = solve_target(
        source,
        EndpointFrame::Identity,
        EndpointFrame::LinkedMirror { mapping },
        &relation,
        Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
    )
    .unwrap();

    assert!((target_authority.at.x - 4.2).abs() < TOLERANCE);
    assert!((target_authority.at.y - 14.4).abs() < TOLERANCE);
    assert!((target_authority.rotation - 161.86989764584402).abs() < TOLERANCE);
    let visible_target = EndpointFrame::LinkedMirror { mapping }
        .project(target_authority)
        .unwrap();
    assert!((visible_target.at.x - 8.0).abs() < TOLERANCE);
    assert!((visible_target.at.y - 2.0).abs() < TOLERANCE);
    assert!((visible_target.rotation - 55.0).abs() < TOLERANCE);
}

#[test]
fn live_linked_source_drives_an_identity_local_target() {
    let mapping = Pose2 {
        at: Vec2 { x: 20.0, y: -7.0 },
        rotation: 36.86989764584402,
    };
    let source_authority = Pose2 {
        at: Vec2 { x: 2.0, y: 5.0 },
        rotation: 25.0,
    };
    let relation = ConstraintRelation::Offset {
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    };

    let target = solve_target(
        source_authority,
        EndpointFrame::LinkedMirror { mapping },
        EndpointFrame::Identity,
        &relation,
        Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
    )
    .unwrap();

    assert!((target.at.x - 20.4).abs() < TOLERANCE);
    assert!((target.at.y + 6.2).abs() < TOLERANCE);
    assert!((target.rotation - 206.86989764584402).abs() < TOLERANCE);
}

#[test]
fn frame_projection_rejects_non_finite_pair_or_pose_values() {
    let frame = EndpointFrame::LinkedMirror {
        mapping: Pose2 {
            at: Vec2 {
                x: f64::NAN,
                y: 0.0,
            },
            rotation: 0.0,
        },
    };
    let pose = Pose2 {
        at: Vec2 { x: 0.0, y: 0.0 },
        rotation: 0.0,
    };

    assert!(frame.project(pose).is_err());
    assert!(
        EndpointFrame::Identity
            .project(Pose2 {
                at: Vec2 {
                    x: 0.0,
                    y: f64::INFINITY
                },
                rotation: 0.0,
            })
            .is_err()
    );
}

#[test]
fn inverse_linked_heading_chooses_the_equivalent_nearest_previous_heading() {
    let frame = EndpointFrame::LinkedMirror {
        mapping: Pose2 {
            at: Vec2 { x: 20.0, y: -7.0 },
            rotation: 36.86989764584402,
        },
    };
    let viewed = Pose2 {
        at: Vec2 { x: 8.0, y: 2.0 },
        rotation: 55.0,
    };

    let unwrapped = frame
        .unproject(
            viewed,
            Pose2 {
                at: Vec2 { x: 4.2, y: 14.4 },
                rotation: 521.869897645844,
            },
        )
        .unwrap();

    assert!((unwrapped.rotation - 521.869897645844).abs() < TOLERANCE);
}

#[test]
fn a_mirrored_key_turns_its_heading_without_the_independent_half_turn() {
    let frame = EndpointFrame::LinkedKey {
        mapping: pair_map(),
    };
    let source = Pose2 {
        at: Vec2 { x: 2.0, y: 5.0 },
        rotation: 25.0,
    };
    let visible = frame.project(source).unwrap();
    assert_close(visible, 15.4, -4.2, 11.86989764584402);
    // The independent rule differs by exactly half a turn.
    let independent = EndpointFrame::LinkedMirror {
        mapping: pair_map(),
    }
    .project(source)
    .unwrap();
    assert_close(independent, 15.4, -4.2, 191.86989764584402);
    assert_close(frame.unproject(visible, source).unwrap(), 2.0, 5.0, 25.0);
}

#[test]
fn companion_rules_project_and_invert_to_the_design_oracles() {
    let companion = Pose2 {
        at: Vec2 { x: 4.0, y: 8.0 },
        rotation: 30.0,
    };
    let key_at = |rotation: f64| Pose2 {
        at: Vec2 { x: 2.0, y: 5.0 },
        rotation,
    };
    let cases = [
        (CompanionRule::Literal, 0.0, 12.0, -3.0, 6.86989764584402),
        (CompanionRule::Stabiliser, 0.0, 12.0, -3.0, 30.0),
        (CompanionRule::Stabiliser, 90.0, 12.0, -3.0, 30.0),
        (CompanionRule::Duplicate, 0.0, 15.2, -0.6, 66.86989764584402),
        (
            CompanionRule::Duplicate,
            90.0,
            15.6,
            -7.8,
            -113.13010235415598,
        ),
        (
            CompanionRule::Tweaked {
                u: 1.0,
                v: -2.0,
                w: 7.0,
            },
            0.0,
            14.0,
            -4.0,
            13.86989764584402,
        ),
        (
            CompanionRule::Tweaked {
                u: 1.0,
                v: -2.0,
                w: 7.0,
            },
            90.0,
            11.0,
            -5.0,
            13.86989764584402,
        ),
    ];
    for (rule, key_heading, x, y, heading) in cases {
        let frame = EndpointFrame::LinkedCompanion {
            mapping: pair_map(),
            key: Some(key_at(key_heading)),
            rule,
        };
        let visible = frame.project(companion).unwrap();
        assert_close(visible, x, y, heading);
        assert_close(frame.unproject(visible, companion).unwrap(), 4.0, 8.0, 30.0);
    }
}

/// A rule that reads the key must not run against an origin it was never given.
#[test]
fn a_companion_rule_that_needs_its_key_refuses_to_run_without_one() {
    let pose = Pose2 {
        at: Vec2 { x: 4.0, y: 8.0 },
        rotation: 30.0,
    };
    for rule in [
        CompanionRule::Duplicate,
        CompanionRule::Tweaked {
            u: 1.0,
            v: -2.0,
            w: 7.0,
        },
    ] {
        let unbound = EndpointFrame::LinkedCompanion {
            mapping: pair_map(),
            key: None,
            rule,
        };
        let projected = unbound.project(pose).unwrap_err();
        assert!(projected.contains("key"), "{projected}");
        assert!(unbound.unproject(pose, pose).is_err());
    }
    // The literal and stabiliser rules do not read the key, so they stand without one.
    for rule in [CompanionRule::Literal, CompanionRule::Stabiliser] {
        let unbound = EndpointFrame::LinkedCompanion {
            mapping: pair_map(),
            key: None,
            rule,
        };
        assert!(unbound.project(pose).is_ok());
    }
}

/// Companion headings and positions are persisted and hashed once a framed record is
/// stored, so their arithmetic is pinned bit for bit, not to a tolerance. The patterns were
/// captured from the 58e4f7bc4 implementation (the last one written rule by rule, before
/// `CompanionRule::frame`), forward then inverse, over inputs chosen so that a regrouped
/// sum or a folded zero shows: ordinary decimals, negative zero in the pose, the key and the
/// tweak, and a pair map with a negative-zero translation. They are not recomputed from the
/// code under test.
#[test]
fn companion_rules_are_bit_pinned_to_the_arithmetic_that_was_reviewed() {
    const PINNED: [(u64, u64, u64); 240] = [
        (0x4028000000000000, 0xc008000000000000, 0x401b7ac672cf11e0),
        (0x400e666666666668, 0x403599999999999a, 0x401b7ac672cf11e0),
        (0x4034000000000000, 0xc01c000000000000, 0x40426f58ce59e23c),
        (0x402799999999999a, 0x403199999999999a, 0x40426f58ce59e23c),
        (0x4033800000000000, 0xc01a000000000000, 0xc055a582b372cd59),
        (0x402699999999999a, 0x403219999999999a, 0xc055a582b372cd59),
        (0x403551eb851eb852, 0xc00a147ae147ae14, 0x405497ac672cf11e),
        (0x402a3d70a3d70a3e, 0x4035570a3d70a3d7, 0x405497ac672cf11e),
        (0x4034000000000000, 0xc01c000000000000, 0x40426f58ce59e23c),
        (0x402799999999999a, 0x403199999999999a, 0x40426f58ce59e23c),
        (0x4028000000000000, 0xc008000000000000, 0x401b7ac672cf11e0),
        (0x400e666666666668, 0x403599999999999a, 0x401b7ac672cf11e0),
        (0x4034000000000000, 0xc01c000000000000, 0x40426f58ce59e23c),
        (0x402799999999999a, 0x403199999999999a, 0x40426f58ce59e23c),
        (0x4033800000000000, 0xc01a000000000000, 0xc055a582b372cd59),
        (0x402699999999999a, 0x403219999999999a, 0xc055a582b372cd59),
        (0x403551eb851eb852, 0xc00a147ae147ae14, 0x405497ac672cf11e),
        (0x402a3d70a3d70a3e, 0x4035570a3d70a3d7, 0x405497ac672cf11e),
        (0x4034000000000000, 0xc01c000000000000, 0x40426f58ce59e23c),
        (0x402799999999999a, 0x403199999999999a, 0x40426f58ce59e23c),
        (0x4028000000000000, 0xc008000000000000, 0x403e000000000000),
        (0x400e666666666668, 0x403599999999999a, 0x403e000000000000),
        (0x4034000000000000, 0xc01c000000000000, 0x8000000000000000),
        (0x402799999999999a, 0x403199999999999a, 0x0000000000000000),
        (0x4033800000000000, 0xc01a000000000000, 0x405edd2f1a9fbe77),
        (0x402699999999999a, 0x403219999999999a, 0x405edd2f1a9fbe77),
        (0x403551eb851eb852, 0xc00a147ae147ae14, 0xc046c00000000000),
        (0x402a3d70a3d70a3e, 0x4035570a3d70a3d7, 0xc046c00000000000),
        (0x4034000000000000, 0xc01c000000000000, 0x0000000000000000),
        (0x402799999999999a, 0x403199999999999a, 0x0000000000000000),
        (0x4028000000000000, 0xc008000000000000, 0x403e000000000000),
        (0x400e666666666668, 0x403599999999999a, 0x403e000000000000),
        (0x4034000000000000, 0xc01c000000000000, 0x8000000000000000),
        (0x402799999999999a, 0x403199999999999a, 0x0000000000000000),
        (0x4033800000000000, 0xc01a000000000000, 0x405edd2f1a9fbe77),
        (0x402699999999999a, 0x403219999999999a, 0x405edd2f1a9fbe77),
        (0x403551eb851eb852, 0xc00a147ae147ae14, 0xc046c00000000000),
        (0x402a3d70a3d70a3e, 0x4035570a3d70a3d7, 0xc046c00000000000),
        (0x4034000000000000, 0xc01c000000000000, 0x0000000000000000),
        (0x402799999999999a, 0x403199999999999a, 0x0000000000000000),
        (0x40373734fb392768, 0x3fec47a4519ab8c8, 0x403f54742c0fed6e),
        (0xc02e173384f095a6, 0x402f1ee17a97b089, 0x403cab8bd3f01292),
        (0x40336703cc6a384e, 0xc01cd3e6f82dbbc2, 0x3ff54742c0fed6e0),
        (0xc0333adb5a0f8bef, 0x401e9f08445ef23a, 0xbff54742c0fed6e0),
        (0x40337c72ea4f8885, 0xc01a04eb301e8f3c, 0x405f324c25a3b9d2),
        (0xc0331d1acc98b48b, 0x4020b4a19d320d54, 0x405e88120f9bc31c),
        (0x40300d5eff5e1f8a, 0xc01456230dfc30c8, 0xc04615c5e9f80949),
        (0xc0367a5ba3e48e26, 0x4023dcd3a0ed8243, 0xc0476a3a1607f6b7),
        (0x40336703cc6a384e, 0xc01cd3e6f82dbbc2, 0x3ff54742c0fed6e0),
        (0xc0333adb5a0f8bef, 0x401e9f08445ef23a, 0xbff54742c0fed6e0),
        (0x4032666666666666, 0x3ffcccccccccccd0, 0x4050b7ac672cf11e),
        (0xc00e666666666668, 0x403599999999999a, 0xc01b7ac672cf11e0),
        (0x4034000000000000, 0xc01c000000000000, 0x40426f58ce59e23c),
        (0xc02799999999999a, 0x403199999999999a, 0xc0426f58ce59e23c),
        (0x4033a8f5c28f5c29, 0xc019851eb851eb85, 0x40640a6dc0e657ca),
        (0xc02699999999999a, 0x403219999999999a, 0x4055a582b372cd59),
        (0x40300a3d70a3d70a, 0xc01ce147ae147ae1, 0xc021429cc6987710),
        (0xc02a3d70a3d70a3e, 0x4035570a3d70a3d7, 0xc05497ac672cf11e),
        (0x4034000000000000, 0xc01c000000000000, 0x40426f58ce59e23c),
        (0xc02799999999999a, 0x403199999999999a, 0xc0426f58ce59e23c),
        (0x402b32e22fd3ac38, 0xc012402fea537730, 0x402bbd63396788f0),
        (0x4010914d2653cc0a, 0x4037cf4d7069cb81, 0x402bbd63396788f0),
        (0x4035997117e9d61c, 0xc0212017f529bb98, 0x4045ef58ce59e23c),
        (0x402848a69329e605, 0x4033cf4d7069cb81, 0x4045ef58ce59e23c),
        (0x4035197117e9d61c, 0xc0202017f529bb98, 0xc053e582b372cd59),
        (0x402748a69329e604, 0x40344f4d7069cb80, 0xc053e582b372cd59),
        (0x4036eb5c9d088e6e, 0xc0134a6d5af74e3a, 0x405657ac672cf11e),
        (0x402aec7d9d6756aa, 0x40378cbe1440d5be, 0x405657ac672cf11e),
        (0x4035997117e9d61c, 0xc0212017f529bb98, 0x4045ef58ce59e23c),
        (0x402848a69329e605, 0x4033cf4d7069cb81, 0x4045ef58ce59e23c),
        (0x402c000000000000, 0xc010000000000000, 0x402bbd63396788f0),
        (0x4013333333333334, 0x403799999999999a, 0x402bbd63396788f0),
        (0x4036000000000000, 0xc020000000000000, 0x4045ef58ce59e23c),
        (0x402999999999999a, 0x403399999999999a, 0x4045ef58ce59e23c),
        (0x4035800000000000, 0xc01e000000000000, 0xc053e582b372cd59),
        (0x402899999999999a, 0x4034199999999999, 0xc053e582b372cd59),
        (0x403751eb851eb852, 0xc0110a3d70a3d70a, 0x405657ac672cf11e),
        (0x402c3d70a3d70a3f, 0x4037570a3d70a3d7, 0x405657ac672cf11e),
        (0x4036000000000000, 0xc020000000000000, 0x4045ef58ce59e23c),
        (0x402999999999999a, 0x403399999999999a, 0x4045ef58ce59e23c),
        (0x4028a1e5d07fbd95, 0xc007f87be888d2f3, 0x401be12cd9357846),
        (0x4010387dbb09bcd8, 0x4035c96aef67345e, 0x401be12cd9357848),
        (0x403450f2e83fdeca, 0xc01bfc3df444697a, 0x40427c259b26af09),
        (0x40281c3edd84de6c, 0x4031c96aef67345e, 0x40427c259b26af09),
        (0x4033d0f2e83fdeca, 0xc019fc3df444697a, 0xc0559f1c4d0c66f3),
        (0x40271c3edd84de6d, 0x4032496aef67345e, 0xc0559f1c4d0c66f2),
        (0x4035a2de6d5e971c, 0xc00a0cf6c9d08107, 0x40549e12cd935784),
        (0x402ac015e7c24f10, 0x403586db933e3e9c, 0x40549e12cd935784),
        (0x403450f2e83fdeca, 0xc01bfc3df444697a, 0x40427c259b26af09),
        (0x40281c3edd84de6c, 0x4031c96aef67345e, 0x40427c259b26af09),
        (0x402899999999999a, 0xc007333333333333, 0x401be12cd9357846),
        (0x4010666666666668, 0x4035b33333333334, 0x401be12cd9357848),
        (0x40344ccccccccccd, 0xc01b99999999999a, 0x40427c259b26af09),
        (0x4028333333333334, 0x4031b33333333333, 0x40427c259b26af09),
        (0x4033cccccccccccd, 0xc01999999999999a, 0xc0559f1c4d0c66f3),
        (0x4027333333333334, 0x4032333333333333, 0xc0559f1c4d0c66f2),
        (0x40359eb851eb851f, 0xc00947ae147ae147, 0x40549e12cd935784),
        (0x402ad70a3d70a3d8, 0x403570a3d70a3d71, 0x40549e12cd935784),
        (0x40344ccccccccccd, 0xc01b99999999999a, 0x40427c259b26af09),
        (0x4028333333333334, 0x4031b33333333333, 0x40427c259b26af09),
        (0x4028000000000000, 0xc008000000000000, 0x401b7ac672cf11e0),
        (0x400e666666666668, 0x403599999999999a, 0x401b7ac672cf11e0),
        (0x4034000000000000, 0xc01c000000000000, 0x40426f58ce59e23c),
        (0x402799999999999a, 0x403199999999999a, 0x40426f58ce59e23c),
        (0x4033800000000000, 0xc01a000000000000, 0xc055a582b372cd59),
        (0x402699999999999a, 0x403219999999999a, 0xc055a582b372cd59),
        (0x403551eb851eb852, 0xc00a147ae147ae14, 0x405497ac672cf11e),
        (0x402a3d70a3d70a3e, 0x4035570a3d70a3d7, 0x405497ac672cf11e),
        (0x4034000000000000, 0xc01c000000000000, 0x40426f58ce59e23c),
        (0x402799999999999a, 0x403199999999999a, 0x40426f58ce59e23c),
        (0x4028000000000000, 0xc008000000000000, 0x401b7ac672cf11e0),
        (0x400e666666666668, 0x403599999999999a, 0x401b7ac672cf11e0),
        (0x4034000000000000, 0xc01c000000000000, 0x40426f58ce59e23c),
        (0x402799999999999a, 0x403199999999999a, 0x40426f58ce59e23c),
        (0x4033800000000000, 0xc01a000000000000, 0xc055a582b372cd59),
        (0x402699999999999a, 0x403219999999999a, 0xc055a582b372cd59),
        (0x403551eb851eb852, 0xc00a147ae147ae14, 0x405497ac672cf11e),
        (0x402a3d70a3d70a3e, 0x4035570a3d70a3d7, 0x405497ac672cf11e),
        (0x4034000000000000, 0xc01c000000000000, 0x40426f58ce59e23c),
        (0x402799999999999a, 0x403199999999999a, 0x40426f58ce59e23c),
        (0xc010000000000000, 0x4020000000000000, 0xc03e000000000000),
        (0xc010000000000000, 0x4020000000000000, 0xc03e000000000000),
        (0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xbfb999999999999a, 0x3fe6666666666666, 0xc05edd2f1a9fbe77),
        (0xbfb999999999999a, 0x3fe6666666666666, 0xc05edd2f1a9fbe77),
        (0x400a666666666666, 0x400199999999999a, 0x4046c00000000000),
        (0x400a666666666666, 0x400199999999999a, 0x4046c00000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xc010000000000000, 0x4020000000000000, 0xc03e000000000000),
        (0xc010000000000000, 0x4020000000000000, 0xc03e000000000000),
        (0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xbfb999999999999a, 0x3fe6666666666666, 0xc05edd2f1a9fbe77),
        (0xbfb999999999999a, 0x3fe6666666666666, 0xc05edd2f1a9fbe77),
        (0x400a666666666666, 0x400199999999999a, 0x4046c00000000000),
        (0x400a666666666666, 0x400199999999999a, 0x4046c00000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xc010000000000000, 0x4020000000000000, 0x403e000000000000),
        (0xc010000000000000, 0x4020000000000000, 0x403e000000000000),
        (0x0000000000000000, 0x0000000000000000, 0x8000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xbfb999999999999a, 0x3fe6666666666666, 0x405edd2f1a9fbe77),
        (0xbfb999999999999a, 0x3fe6666666666666, 0x405edd2f1a9fbe77),
        (0x400a666666666666, 0x400199999999999a, 0xc046c00000000000),
        (0x400a666666666666, 0x400199999999999a, 0xc046c00000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xc010000000000000, 0x4020000000000000, 0x403e000000000000),
        (0xc010000000000000, 0x4020000000000000, 0x403e000000000000),
        (0x0000000000000000, 0x0000000000000000, 0x8000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xbfb999999999999a, 0x3fe6666666666666, 0x405edd2f1a9fbe77),
        (0xbfb999999999999a, 0x3fe6666666666666, 0x405edd2f1a9fbe77),
        (0x400a666666666666, 0x400199999999999a, 0xc046c00000000000),
        (0x400a666666666666, 0x400199999999999a, 0xc046c00000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x401d3609106f4bf8, 0x401182aae06cb3e6, 0xc01628f5c28f5c28),
        (0xbfe960fd8b58e09e, 0x40220e3870dd8567, 0x4050628f5c28f5c2),
        (0xbfe3458d0309170e, 0x3fc8b49041f5eeba, 0xc041c51eb851eb85),
        (0x3fe3458d0309170e, 0x3fc8b49041f5eeba, 0x4041c51eb851eb85),
        (0xbfbd2dbef8284516, 0x3fe68b1a06122e1d, 0x4055fa9fbe76c8b4),
        (0x3fd1b5cd173ef13b, 0x3fea4374d321d918, 0x4063dfdf3b645a1d),
        (0xc00011ca14d43d79, 0x400f35ffeca8b427, 0xc054428f5c28f5c2),
        (0xc00ae4f6fb25de58, 0x3fb0a25a1cee322a, 0xc023eb851eb851ec),
        (0xbfe3458d0309170e, 0x3fc8b49041f5eeba, 0xc041c51eb851eb85),
        (0x3fe3458d0309170e, 0x3fc8b49041f5eeba, 0x4041c51eb851eb85),
        (0x4010000000000000, 0x4020000000000000, 0x403e000000000000),
        (0x4010000000000000, 0x4020000000000000, 0x403e000000000000),
        (0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x3fb999999999999a, 0x3fe6666666666666, 0x405edd2f1a9fbe77),
        (0x3fb999999999999a, 0x3fe6666666666666, 0x405edd2f1a9fbe77),
        (0xc00a666666666666, 0x400199999999999a, 0xc046c00000000000),
        (0xc00a666666666666, 0x400199999999999a, 0xc046c00000000000),
        (0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xc00d43cc19bece54, 0x40172930a4bf3864, 0xc037000000000000),
        (0xc00d43cc19bece54, 0x40246b67ada063ce, 0xc037000000000000),
        (0x3fd5e19f32098d64, 0xc001ad9eb6818f39, 0x401c000000000000),
        (0x3fd5e19f32098d64, 0x4001ad9eb6818f39, 0x401c000000000000),
        (0x3fcef67197464dfb, 0xbff8280a39cfeb3f, 0xc05d1d2f1a9fbe77),
        (0x3fcef67197464dfb, 0x40074738501b28d2, 0xc05d1d2f1a9fbe77),
        (0x400d229a4ca79812, 0xbf84051ce7f59f00, 0x404a400000000000),
        (0x400d229a4ca79812, 0x4011a39c280d946a, 0x404a400000000000),
        (0x3fd5e19f32098d64, 0xc001ad9eb6818f39, 0x401c000000000000),
        (0x3fd5e19f32098d64, 0x4001ad9eb6818f39, 0x401c000000000000),
        (0xc008000000000000, 0x4018000000000000, 0xc037000000000000),
        (0xc008000000000000, 0x4024000000000000, 0xc037000000000000),
        (0x3ff0000000000000, 0xc000000000000000, 0x401c000000000000),
        (0x3ff0000000000000, 0x4000000000000000, 0x401c000000000000),
        (0x3feccccccccccccd, 0xbff4cccccccccccd, 0xc05d1d2f1a9fbe77),
        (0x3feccccccccccccd, 0x400599999999999a, 0xc05d1d2f1a9fbe77),
        (0x4011333333333333, 0x3fc99999999999a0, 0x404a400000000000),
        (0x4011333333333333, 0x4010cccccccccccd, 0x404a400000000000),
        (0x3ff0000000000000, 0xc000000000000000, 0x401c000000000000),
        (0x3ff0000000000000, 0x4000000000000000, 0x401c000000000000),
        (0xc00df56af052ecb6, 0x401f40baa8c994ec, 0xc03de66666666666),
        (0xc00df56af052ecb6, 0x40205fa2ab9b358a, 0xc03de66666666666),
        (0x3fd054a87d689a51, 0xbfc7e8aae6cd627c, 0x3fb999999999999a),
        (0x3fd054a87d689a51, 0x3fc7e8aae6cd627c, 0x3fb999999999999a),
        (0x3fc3dc842e0467d5, 0x3fe06c3bacb30dc7, 0xc05ed6c8b4395811),
        (0x3fc3dc842e0467d5, 0x3fec60912019bf05, 0xc05ed6c8b4395811),
        (0x400c70fb761379b0, 0x40001b0eeb2cc372, 0x4046cccccccccccd),
        (0x400c70fb761379b0, 0x4003182448066fc2, 0x4046cccccccccccd),
        (0x3fd054a87d689a51, 0xbfc7e8aae6cd627c, 0x3fb999999999999a),
        (0x3fd054a87d689a51, 0x3fc7e8aae6cd627c, 0x3fb999999999999a),
        (0xc00d99999999999a, 0x401f99999999999a, 0xc03de66666666666),
        (0xc00d99999999999a, 0x4020333333333333, 0xc03de66666666666),
        (0x3fd3333333333333, 0xbfb999999999999a, 0x3fb999999999999a),
        (0x3fd3333333333333, 0x3fb999999999999a, 0x3fb999999999999a),
        (0x3fc9999999999999, 0x3fe3333333333333, 0xc05ed6c8b4395811),
        (0x3fc9999999999999, 0x3fe9999999999999, 0xc05ed6c8b4395811),
        (0x400ccccccccccccc, 0x4000cccccccccccd, 0x4046cccccccccccd),
        (0x400ccccccccccccc, 0x4002666666666667, 0x4046cccccccccccd),
        (0x3fd3333333333333, 0xbfb999999999999a, 0x3fb999999999999a),
        (0x3fd3333333333333, 0x3fb999999999999a, 0x3fb999999999999a),
        (0xc010000000000000, 0x4020000000000000, 0xc03e000000000000),
        (0xc010000000000000, 0x4020000000000000, 0xc03e000000000000),
        (0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xbfb999999999999a, 0x3fe6666666666666, 0xc05edd2f1a9fbe77),
        (0xbfb999999999999a, 0x3fe6666666666666, 0xc05edd2f1a9fbe77),
        (0x400a666666666666, 0x400199999999999a, 0x4046c00000000000),
        (0x400a666666666666, 0x400199999999999a, 0x4046c00000000000),
        (0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xc010000000000000, 0x4020000000000000, 0xc03e000000000000),
        (0xc010000000000000, 0x4020000000000000, 0xc03e000000000000),
        (0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0xbfb999999999999a, 0x3fe6666666666666, 0xc05edd2f1a9fbe77),
        (0xbfb999999999999a, 0x3fe6666666666666, 0xc05edd2f1a9fbe77),
        (0x400a666666666666, 0x400199999999999a, 0x4046c00000000000),
        (0x400a666666666666, 0x400199999999999a, 0x4046c00000000000),
        (0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
        (0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
    ];
    let p = |x: f64, y: f64, rotation: f64| Pose2 {
        at: Vec2 { x, y },
        rotation,
    };
    let mappings = [p(20.0, -7.0, 36.86989764584402), p(-0.0, -0.0, 0.0)];
    let rules = [
        CompanionRule::Literal,
        CompanionRule::Stabiliser,
        CompanionRule::Duplicate,
        CompanionRule::Tweaked {
            u: 1.0,
            v: -2.0,
            w: 7.0,
        },
        CompanionRule::Tweaked {
            u: 0.3,
            v: -0.1,
            w: 0.1,
        },
        CompanionRule::Tweaked {
            u: -0.0,
            v: -0.0,
            w: -0.0,
        },
    ];
    let poses = [
        p(4.0, 8.0, 30.0),
        p(-0.0, -0.0, -0.0),
        p(0.1, 0.7, 123.456),
        p(-3.3, 2.2, -45.5),
        p(0.0, 0.0, 0.0),
    ];
    let keys = [p(0.3, 0.1, 17.77), p(-0.0, -0.0, -0.0)];
    let previous = p(4.0, 8.0, 30.0);
    let mut pinned = PINNED.iter();
    for mapping in mappings {
        for rule in rules {
            for key in keys {
                for pose in poses {
                    let frame = EndpointFrame::LinkedCompanion {
                        mapping,
                        key: Some(key),
                        rule,
                    };
                    for result in [
                        frame.project(pose).unwrap(),
                        frame.unproject(pose, previous).unwrap(),
                    ] {
                        let bits = (
                            result.at.x.to_bits(),
                            result.at.y.to_bits(),
                            result.rotation.to_bits(),
                        );
                        assert_eq!(
                            bits,
                            *pinned.next().unwrap(),
                            "{mapping:?} {rule:?} key {key:?} pose {pose:?}: {result:?}"
                        );
                    }
                }
            }
        }
    }
}
