use crate::model::{Constraint, MirrorAxis, Pose2, Vec2};
use serde::{Deserialize, Serialize};

/// A directional relation evaluated between the source and target endpoint views.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum ConstraintRelation {
    /// `offset` is a vector along the board's axes, added to the source position.
    Offset {
        offset: Vec2,
        rotation: f64,
    },
    /// `offset` runs along the source's own axes: it is turned by the source's heading before
    /// it is added, so the target keeps its pose in the source's frame (ADR-0011).
    PartOffset {
        offset: Vec2,
        rotation: f64,
    },
    MirrorLine {
        at: Vec2,
        angle: f64,
    },
}

#[cfg_attr(not(test), expect(dead_code))] // `from_legacy` and `inverse` have no production caller yet: the compile-all-records pass and the drag seam land with 114/115.
impl ConstraintRelation {
    /// The relation a stored record states, so every record kind is compiled by one path.
    /// A vertical mirror at `c` is the line through `(c, 0)` heading 90; a horizontal one is
    /// the line through `(0, c)` heading 0. The heading rules are bit-equal to the stored ones;
    /// the position is only equal to within rounding, because reflection through a unit normal
    /// does not reproduce `2c - x` bit for bit and may nudge the coordinate along the axis.
    /// Saved Mirror records therefore stay on the stored arithmetic: this compiled path must
    /// not drive them until an exact pin exists.
    pub(super) fn from_legacy(constraint: &Constraint) -> Self {
        match constraint {
            Constraint::Offset {
                offset, rotation, ..
            } => Self::Offset {
                offset: *offset,
                rotation: *rotation,
            },
            Constraint::PartOffset {
                offset, rotation, ..
            } => Self::PartOffset {
                offset: *offset,
                rotation: *rotation,
            },
            Constraint::Mirror {
                axis, coordinate, ..
            } => match axis {
                MirrorAxis::Vertical => Self::MirrorLine {
                    at: Vec2 {
                        x: *coordinate,
                        y: 0.0,
                    },
                    angle: 90.0,
                },
                MirrorAxis::Horizontal => Self::MirrorLine {
                    at: Vec2 {
                        x: 0.0,
                        y: *coordinate,
                    },
                    angle: 0.0,
                },
            },
        }
    }

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
            Self::PartOffset { offset, rotation } => super::follow_pose(
                source,
                Pose2 {
                    at: *offset,
                    rotation: *rotation,
                },
            ),
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
            Self::PartOffset { offset, rotation } => {
                // The source heading is the target's minus the stored turn, so the turned
                // offset can be taken back out without the source pose.
                let source_rotation = target.rotation - rotation;
                let turned =
                    crate::matrix::splay::rotate(*offset, source_rotation, Vec2::default());
                Pose2 {
                    at: Vec2 {
                        x: target.at.x - turned.x,
                        y: target.at.y - turned.y,
                    },
                    rotation: source_rotation,
                }
            }
            Self::MirrorLine { .. } => self.apply(target)?,
        };

        validate_pose(result)?;
        Ok(result)
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        let finite = match self {
            Self::Offset { offset, rotation } | Self::PartOffset { offset, rotation } => {
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
    use crate::model::{Constraint, MirrorAxis, Pose2, Vec2};
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

    #[test]
    fn part_offset_turns_the_offset_by_the_source_heading_before_adding_it() {
        let relation = ConstraintRelation::PartOffset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        };
        // Heading 90: (5, -2) turns to (2, 5).
        let quarter = Pose2 {
            at: Vec2 { x: 3.0, y: 4.0 },
            rotation: 90.0,
        };
        assert_pose_close(
            relation.apply(quarter).unwrap(),
            Pose2 {
                at: Vec2 { x: 5.0, y: 9.0 },
                rotation: 105.0,
            },
        );
        // Heading 30: x = 5cos30 + 2sin30, y = 5sin30 - 2cos30.
        let thirty = Pose2 {
            at: Vec2 { x: 3.0, y: 4.0 },
            rotation: 30.0,
        };
        assert_pose_close(
            relation.apply(thirty).unwrap(),
            Pose2 {
                at: Vec2 {
                    x: 8.330127018922193,
                    y: 4.767949192431122,
                },
                rotation: 45.0,
            },
        );
        // The same numbers as a board-axes offset do not turn with the source.
        let board = ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        };
        assert_pose_close(
            board.apply(quarter).unwrap(),
            Pose2 {
                at: Vec2 { x: 8.0, y: 2.0 },
                rotation: 105.0,
            },
        );
        for source in [quarter, thirty] {
            assert_pose_close(
                relation.inverse(relation.apply(source).unwrap()).unwrap(),
                source,
            );
        }
    }

    #[test]
    fn legacy_records_compile_to_relations_with_literal_results() {
        let source = Pose2 {
            at: Vec2 { x: 2.0, y: 5.0 },
            rotation: 25.0,
        };
        let cases = [
            (
                Constraint::Offset {
                    id: "o".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    offset: Vec2 { x: 10.0, y: 2.0 },
                    rotation: 5.0,
                },
                Pose2 {
                    at: Vec2 { x: 12.0, y: 7.0 },
                    rotation: 30.0,
                },
            ),
            (
                // Heading 25 turns (10, 2) to (10cos25 - 2sin25, 10sin25 + 2cos25).
                Constraint::PartOffset {
                    id: "p".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    offset: Vec2 { x: 10.0, y: 2.0 },
                    rotation: 5.0,
                },
                Pose2 {
                    at: Vec2 {
                        x: 10.2178413468851,
                        y: 11.038798191480295,
                    },
                    rotation: 30.0,
                },
            ),
            (
                Constraint::Mirror {
                    id: "v".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    axis: MirrorAxis::Vertical,
                    coordinate: 10.0,
                },
                Pose2 {
                    at: Vec2 { x: 18.0, y: 5.0 },
                    rotation: 155.0,
                },
            ),
            (
                Constraint::Mirror {
                    id: "h".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    axis: MirrorAxis::Horizontal,
                    coordinate: 3.0,
                },
                Pose2 {
                    at: Vec2 { x: 2.0, y: 1.0 },
                    rotation: -25.0,
                },
            ),
        ];
        for (constraint, expected) in cases {
            let relation = ConstraintRelation::from_legacy(&constraint);
            assert_pose_close(relation.apply(source).unwrap(), expected);
        }
    }

    /// A vertical mirror's compiled position differs from the stored `2c - x` in the last
    /// bits for ordinary values, and its heading does not. This is the hazard the
    /// `from_legacy` note names; the day an exact pin exists this test is the one to flip.
    #[test]
    fn compiled_mirror_position_is_close_but_not_bit_equal_to_the_stored_rule() {
        let relation = ConstraintRelation::from_legacy(&Constraint::Mirror {
            id: "v".into(),
            source_part_id: "a".into(),
            target_part_id: "b".into(),
            axis: MirrorAxis::Vertical,
            coordinate: 26.377,
        });
        let source = Pose2 {
            at: Vec2 {
                x: -73.127,
                y: 69.487,
            },
            rotation: 35.0,
        };
        let result = relation.apply(source).unwrap();
        let stored_x = 2.0 * 26.377 - source.at.x;
        assert!((result.at.x - stored_x).abs() < 1e-12);
        assert_ne!(result.at.x, stored_x);
        assert!((result.at.y - source.at.y).abs() < 1e-12);
        assert_eq!(result.rotation, 180.0 - source.rotation);
    }

    #[test]
    fn part_offset_relation_serializes_with_its_own_kind() {
        let relation = ConstraintRelation::PartOffset {
            offset: Vec2 { x: 1.0, y: 2.0 },
            rotation: 3.0,
        };
        assert_eq!(
            serde_json::to_value(&relation).unwrap(),
            json!({"kind": "part-offset", "offset": {"x": 1.0, "y": 2.0}, "rotation": 3.0})
        );
        let bad = ConstraintRelation::PartOffset {
            offset: Vec2 {
                x: f64::NAN,
                y: 0.0,
            },
            rotation: 0.0,
        };
        assert!(
            bad.apply(Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0
            })
            .is_err()
        );
    }
}
