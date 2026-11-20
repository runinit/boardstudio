use boardstudio_model::constraints::endpoints::*;
use boardstudio_model::constraints::relation::ConstraintRelation;
use boardstudio_model::model::{Pose2, Vec2};
use serde_json::json;

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
