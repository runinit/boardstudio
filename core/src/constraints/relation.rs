use crate::model::{Pose2, Vec2};
use serde::{Deserialize, Serialize};

/// A directional relation evaluated between the source and target endpoint views.
#[cfg_attr(not(test), expect(dead_code))] // Consumed by the later private compiler and schema integration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum ConstraintRelation {
    Offset { offset: Vec2, rotation: f64 },
    MirrorLine { at: Vec2, angle: f64 },
}

#[cfg_attr(not(test), expect(dead_code))] // Consumed by later private solver integration.
impl ConstraintRelation {
    pub(super) fn apply(&self, source: Pose2) -> Result<Pose2, String> {
        self.validate()?;
        validate_pose(source)?;

        let result = match self {
            Self::Offset { offset, rotation } => Pose2 {
                at: Vec2 {
                    x: source.at.x + offset.x,
                    y: source.at.y + offset.y,
                },
                rotation: source.rotation + rotation,
            },
            Self::MirrorLine { at, angle } => {
                let tangent = angle.to_radians();
                let (sin, cos) = tangent.sin_cos();
                let normal_x = -sin;
                let normal_y = cos;
                let from_anchor_x = source.at.x - at.x;
                let from_anchor_y = source.at.y - at.y;
                let signed_distance = normal_x * from_anchor_x + normal_y * from_anchor_y;
                Pose2 {
                    at: Vec2 {
                        x: source.at.x - 2.0 * normal_x * signed_distance,
                        y: source.at.y - 2.0 * normal_y * signed_distance,
                    },
                    rotation: 2.0 * angle - source.rotation,
                }
            }
        };

        validate_pose(result)?;
        Ok(result)
    }

    pub(super) fn inverse(&self, target: Pose2) -> Result<Pose2, String> {
        self.validate()?;
        validate_pose(target)?;

        let result = match self {
            Self::Offset { offset, rotation } => Pose2 {
                at: Vec2 {
                    x: target.at.x - offset.x,
                    y: target.at.y - offset.y,
                },
                rotation: target.rotation - rotation,
            },
            Self::MirrorLine { .. } => self.apply(target)?,
        };

        validate_pose(result)?;
        Ok(result)
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        let finite = match self {
            Self::Offset { offset, rotation } => {
                offset.x.is_finite() && offset.y.is_finite() && rotation.is_finite()
            }
            Self::MirrorLine { at, angle } => {
                at.x.is_finite() && at.y.is_finite() && angle.is_finite()
            }
        };
        if finite {
            Ok(())
        } else {
            Err("Constraint relation contains a non-finite value".into())
        }
    }
}

fn validate_pose(pose: Pose2) -> Result<(), String> {
    if pose.at.x.is_finite() && pose.at.y.is_finite() && pose.rotation.is_finite() {
        Ok(())
    } else {
        Err("Constraint relation received or produced a non-finite pose".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Pose2, Vec2};
    use serde_json::json;

    const TOLERANCE: f64 = 1e-9;

    fn assert_pose_close(actual: Pose2, expected: Pose2) {
        assert!((actual.at.x - expected.at.x).abs() < TOLERANCE);
        assert!((actual.at.y - expected.at.y).abs() < TOLERANCE);
        assert!((actual.rotation - expected.rotation).abs() < TOLERANCE);
    }

    #[test]
    fn slanted_reflection_matches_independent_literal_oracle() {
        let relation = ConstraintRelation::MirrorLine {
            at: Vec2 { x: 1.0, y: -2.0 },
            angle: 53.13010235415598,
        };

        let result = relation
            .apply(Pose2 {
                at: Vec2 { x: 2.0, y: 5.0 },
                rotation: 25.0,
            })
            .unwrap();

        assert_pose_close(
            result,
            Pose2 {
                at: Vec2 { x: 7.44, y: 0.92 },
                rotation: 81.26020470831196,
            },
        );
    }

    #[test]
    fn offset_relation_is_world_translation_and_full_heading_addition() {
        let relation = ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        };
        let source = Pose2 {
            at: Vec2 { x: 3.0, y: 4.0 },
            rotation: 350.0,
        };

        assert_pose_close(
            relation.apply(source).unwrap(),
            Pose2 {
                at: Vec2 { x: 8.0, y: 2.0 },
                rotation: 365.0,
            },
        );
        assert_pose_close(
            relation.inverse(relation.apply(source).unwrap()).unwrap(),
            source,
        );
    }

    #[test]
    fn transported_slanted_line_keeps_authored_values_and_commutes_with_pair_map() {
        let original = ConstraintRelation::MirrorLine {
            at: Vec2 { x: 1.0, y: -2.0 },
            angle: 53.13010235415598,
        };
        let equivalent = ConstraintRelation::MirrorLine {
            at: Vec2 { x: 20.4, y: -9.2 },
            angle: 163.73979529168804,
        };
        let source = Pose2 {
            at: Vec2 { x: 2.0, y: 5.0 },
            rotation: 25.0,
        };

        let reflected_source = Pose2 {
            at: Vec2 { x: 15.4, y: -4.2 },
            rotation: 191.86989764584402,
        };
        let reflected_result = Pose2 {
            at: Vec2 {
                x: 13.496,
                y: -10.728,
            },
            rotation: 135.60969293753207,
        };
        assert_pose_close(
            equivalent.apply(reflected_source).unwrap(),
            reflected_result,
        );
        assert_pose_close(
            original
                .inverse(Pose2 {
                    at: Vec2 { x: 7.44, y: 0.92 },
                    rotation: 81.26020470831196,
                })
                .unwrap(),
            source,
        );
        assert_eq!(
            serde_json::to_value(equivalent).unwrap(),
            json!({
                "kind": "mirror-line",
                "at": {"x": 20.4, "y": -9.2},
                "angle": 163.73979529168804
            })
        );
    }

    #[test]
    fn non_finite_relation_or_pose_is_rejected_before_projection() {
        let non_finite_line = ConstraintRelation::MirrorLine {
            at: Vec2 {
                x: 0.0,
                y: f64::INFINITY,
            },
            angle: 45.0,
        };
        let valid_line = ConstraintRelation::MirrorLine {
            at: Vec2 { x: 0.0, y: 0.0 },
            angle: f64::NAN,
        };
        let unrepresentable_axis = ConstraintRelation::MirrorLine {
            at: Vec2 { x: 0.0, y: 0.0 },
            angle: f64::MAX,
        };
        let offset = ConstraintRelation::Offset {
            offset: Vec2 {
                x: f64::NAN,
                y: 0.0,
            },
            rotation: 0.0,
        };
        let source = Pose2 {
            at: Vec2 { x: 1.0, y: 2.0 },
            rotation: 3.0,
        };

        assert!(non_finite_line.apply(source).is_err());
        assert!(valid_line.apply(source).is_err());
        assert!(unrepresentable_axis.apply(source).is_err());
        assert!(offset.apply(source).is_err());
        assert!(
            valid_line
                .apply(Pose2 {
                    at: Vec2 {
                        x: f64::NAN,
                        y: 2.0
                    },
                    rotation: 3.0,
                })
                .is_err()
        );
    }
}
