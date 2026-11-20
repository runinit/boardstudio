use boardstudio_model::constraints::endpoints::EndpointFrame;
use boardstudio_model::constraints::relation::ConstraintRelation;
use boardstudio_model::constraints::retune::*;
use boardstudio_model::model::{Pose2, Vec2};

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
