use boardstudio_model::constraints::endpoints::EndpointFrame;
use boardstudio_model::constraints::relation::ConstraintRelation;
use boardstudio_model::constraints::retune::*;
use boardstudio_model::model::{Pose2, Vec2};

mod common;
use common::{assert_close, pair_map};

const TOLERANCE: f64 = 1e-9;

fn local_source() -> Pose2 {
    Pose2 {
        at: Vec2 { x: 3.0, y: 4.0 },
        rotation: 40.0,
    }
}

#[test]
fn a_reflecting_relation_cannot_be_dragged() {
    let relation = ConstraintRelation::MirrorLine {
        at: Vec2 { x: 0.0, y: 0.0 },
        angle: 90.0,
    };
    let message = retune_target(
        &relation,
        local_source(),
        EndpointFrame::Identity,
        EndpointFrame::Identity,
        Pose2 {
            at: Vec2 { x: -3.0, y: 4.0 },
            rotation: 140.0,
        },
        Vec2 { x: 1.0, y: 1.0 },
        None,
    )
    .unwrap_err();
    assert!(message.contains("mirror"), "{message}");
}

#[test]
fn a_non_finite_request_is_refused() {
    let relation = ConstraintRelation::Offset {
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    };
    assert!(
        retune_target(
            &relation,
            local_source(),
            EndpointFrame::Identity,
            EndpointFrame::Identity,
            Pose2 {
                at: Vec2 { x: 8.0, y: 2.0 },
                rotation: 55.0,
            },
            Vec2 {
                x: f64::NAN,
                y: 0.0
            },
            None,
        )
        .is_err()
    );
}

#[test]
fn dragging_a_linked_target_to_a_visible_point_retunes_the_stored_offset() {
    // Design oracle: the mixed example's visible target (8, 2) is dragged to (12, 1).
    let relation = ConstraintRelation::Offset {
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    };
    let target_frame = EndpointFrame::LinkedMirror {
        mapping: pair_map(),
    };
    let retuned = retune_target(
        &relation,
        local_source(),
        EndpointFrame::Identity,
        target_frame,
        Pose2 {
            at: Vec2 { x: 4.2, y: 14.4 },
            rotation: 161.86989764584402,
        },
        Vec2 { x: 12.0, y: 1.0 },
        None,
    )
    .unwrap();

    assert_close(retuned.target_authority, 1.6, 11.2, 161.86989764584402);
    let ConstraintRelation::Offset { offset, rotation } = retuned.relation else {
        panic!("the relation kind changed");
    };
    assert!((offset.x - 9.0).abs() < TOLERANCE && (offset.y + 3.0).abs() < TOLERANCE);
    // A position drag carries no heading, so the stored turn is left bit for bit.
    assert_eq!(rotation.to_bits(), 15.0_f64.to_bits());
}

#[test]
fn a_part_offset_drag_is_turned_back_into_the_source_frame() {
    let relation = ConstraintRelation::PartOffset {
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    };
    let source = Pose2 {
        at: Vec2 { x: 3.0, y: 4.0 },
        rotation: 90.0,
    };
    // Heading 90 turns (5, -2) to (2, 5): the target sits at (5, 9).
    let retuned = retune_target(
        &relation,
        source,
        EndpointFrame::Identity,
        EndpointFrame::Identity,
        Pose2 {
            at: Vec2 { x: 5.0, y: 9.0 },
            rotation: 105.0,
        },
        Vec2 { x: 7.0, y: 10.0 },
        Some(120.0),
    )
    .unwrap();

    assert_close(retuned.target_authority, 7.0, 10.0, 120.0);
    let ConstraintRelation::PartOffset { offset, rotation } = retuned.relation else {
        panic!("the relation kind changed");
    };
    // (2, 1) in board axes is (1, -2) in the source's axes; the turn follows the heading.
    assert!((offset.x - 6.0).abs() < TOLERANCE && (offset.y + 4.0).abs() < TOLERANCE);
    assert!((rotation - 30.0).abs() < TOLERANCE);
}
