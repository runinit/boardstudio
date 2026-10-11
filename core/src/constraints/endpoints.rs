use crate::constraints::relation::ConstraintRelation;
use crate::mechanical::pair;
use crate::model::Pose2;
use serde::{Deserialize, Serialize};

/// The authored, visible identity of a framed relation endpoint.
#[cfg_attr(not(test), expect(dead_code))] // Consumed by the later private compiler and schema integration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum ConstraintEndpoint {
    Part {
        #[serde(rename = "partId")]
        part_id: String,
    },
    LinkedMirror {
        #[serde(rename = "pairId")]
        pair_id: String,
        #[serde(rename = "sourcePartId")]
        source_part_id: String,
        #[serde(rename = "mirrorPartId")]
        mirror_part_id: String,
    },
}

/// Private wire-shape candidate; the existing global constraint ID stays attached to the record.
#[cfg_attr(not(test), expect(dead_code))] // The model boundary adopts this record in a later cohort.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum FramedConstraint {
    Framed {
        id: String,
        source: ConstraintEndpoint,
        target: ConstraintEndpoint,
        relation: ConstraintRelation,
    },
}

#[cfg_attr(not(test), expect(dead_code))] // The later private compiler reads both endpoint identity spaces.
impl ConstraintEndpoint {
    pub(super) fn visible_part_id(&self) -> &str {
        match self {
            Self::Part { part_id } => part_id,
            Self::LinkedMirror { mirror_part_id, .. } => mirror_part_id,
        }
    }

    pub(super) fn authoritative_part_id(&self) -> &str {
        match self {
            Self::Part { part_id } => part_id,
            Self::LinkedMirror { source_part_id, .. } => source_part_id,
        }
    }
}

/// A live endpoint frame. Linked mappings are supplied by the current pair owner.
#[cfg_attr(not(test), expect(dead_code))] // Used before solver integration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum EndpointFrame {
    Identity,
    LinkedMirror { mapping: Pose2 },
}

#[cfg_attr(not(test), expect(dead_code))] // Consumed by later private solver integration.
impl EndpointFrame {
    pub(super) fn project(self, authoritative: Pose2) -> Result<Pose2, String> {
        validate_pose(authoritative)?;
        let projected = match self {
            Self::Identity => authoritative,
            Self::LinkedMirror { mapping } => {
                validate_mapping(mapping)?;
                Pose2 {
                    at: pair::forward(mapping, authoritative.at),
                    rotation: (mapping.rotation + 180.0 - authoritative.rotation).rem_euclid(360.0),
                }
            }
        };
        validate_pose(projected)?;
        Ok(projected)
    }

    pub(super) fn unproject(
        self,
        visible: Pose2,
        previous_authoritative: Pose2,
    ) -> Result<Pose2, String> {
        validate_pose(visible)?;
        validate_pose(previous_authoritative)?;
        let authoritative = match self {
            Self::Identity => visible,
            Self::LinkedMirror { mapping } => {
                validate_mapping(mapping)?;
                let base_rotation = mapping.rotation + 180.0 - visible.rotation;
                Pose2 {
                    at: pair::inverse(mapping, visible.at),
                    rotation: nearest_equivalent(base_rotation, previous_authoritative.rotation),
                }
            }
        };
        validate_pose(authoritative)?;
        Ok(authoritative)
    }
}

fn nearest_equivalent(angle: f64, reference: f64) -> f64 {
    // f64::round makes exact half-turn ties deterministic, away from zero.
    angle + 360.0 * ((reference - angle) / 360.0).round()
}

fn validate_mapping(mapping: Pose2) -> Result<(), String> {
    validate_pose(mapping)
}

fn validate_pose(pose: Pose2) -> Result<(), String> {
    if pose.at.x.is_finite() && pose.at.y.is_finite() && pose.rotation.is_finite() {
        Ok(())
    } else {
        Err("Constraint endpoint frame contains a non-finite mapping or pose".into())
    }
}

/// Evaluate source → target in their authored endpoint frames, then return target authority.
#[cfg_attr(not(test), expect(dead_code))] // The later solver consumes this directional private seam.
pub(super) fn solve_target(
    source_authority: Pose2,
    source_frame: EndpointFrame,
    target_frame: EndpointFrame,
    relation: &ConstraintRelation,
    previous_target_authority: Pose2,
) -> Result<Pose2, String> {
    let source_view = source_frame.project(source_authority)?;
    let target_view = relation.apply(source_view)?;
    target_frame.unproject(target_view, previous_target_authority)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::relation::ConstraintRelation;
    use crate::model::{Pose2, Vec2};
    use serde_json::json;

    const TOLERANCE: f64 = 1e-9;

    #[test]
    fn linked_mirror_address_keeps_visible_identity_separate_from_authority() {
        let endpoint = ConstraintEndpoint::LinkedMirror {
            pair_id: "pair/A".into(),
            source_part_id: "alpha/T".into(),
            mirror_part_id: "beta/T".into(),
        };

        assert_eq!(endpoint.visible_part_id(), "beta/T");
        assert_eq!(endpoint.authoritative_part_id(), "alpha/T");
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

        assert_eq!(endpoint.visible_part_id(), "local/P");
        assert_eq!(endpoint.authoritative_part_id(), "local/P");
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
                offset: crate::model::Vec2 { x: 5.0, y: -2.0 },
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
}
