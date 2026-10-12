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
    /// the line through `(0, c)` heading 0.
    ///
    /// Bit-exact against the stored solver: `apply` takes the stored arithmetic for a line whose
    /// angle is exactly 90 (position `2c - x`, `y` untouched, heading `180 - r`) or exactly 0
    /// (`x` untouched, `2c - y`, heading `-r`, keeping a zero heading's sign), which is every
    /// line this compiles. Any other angle uses the general reflection, which is equal to the
    /// literal reflection only within rounding, and is never produced from a stored record.
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
            // Axis-aligned lines keep the stored solver's arithmetic exactly (see `from_legacy`).
            Self::MirrorLine { at, angle } if *angle == 90.0 => Pose2 {
                at: Vec2 {
                    x: 2.0 * at.x - source.at.x,
                    y: source.at.y,
                },
                rotation: 180.0 - source.rotation,
            },
            Self::MirrorLine { at, angle } if *angle == 0.0 => Pose2 {
                at: Vec2 {
                    x: source.at.x,
                    y: 2.0 * at.y - source.at.y,
                },
                rotation: -source.rotation,
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

    /// An axis-aligned mirror compiles to the stored arithmetic bit for bit: position
    /// `2c - x` (or `2c - y`) with the other coordinate untouched, and heading `180 - r`
    /// (or `-r`, which keeps the sign of a zero heading).
    #[test]
    fn axis_aligned_mirrors_reproduce_the_stored_solver_bit_for_bit() {
        let sources = [
            (-73.127, 69.487, 35.0),
            (2.0, 5.0, 25.0),
            (0.0, 0.0, 0.0),
            (-0.0, -0.0, -0.0),
            (1e-300, -1e300, 360.0),
            (13.37, -0.1, 179.999),
        ];
        for axis in [MirrorAxis::Vertical, MirrorAxis::Horizontal] {
            for coordinate in [26.377, -3.5, 0.0, -0.0, 4.25, 1e-308, -1e15] {
                let stored = Constraint::Mirror {
                    id: "m".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    axis: axis.clone(),
                    coordinate,
                };
                let relation = ConstraintRelation::from_legacy(&stored);
                for (x, y, rotation) in sources {
                    let source = Pose2 {
                        at: Vec2 { x, y },
                        rotation,
                    };
                    let expected = super::super::constraint_pose(&stored, source);
                    let actual = relation.apply(source).unwrap();
                    assert_eq!(
                        (
                            actual.at.x.to_bits(),
                            actual.at.y.to_bits(),
                            actual.rotation.to_bits()
                        ),
                        (
                            expected.at.x.to_bits(),
                            expected.at.y.to_bits(),
                            expected.rotation.to_bits()
                        ),
                        "{axis:?} at {coordinate} from {source:?}"
                    );
                }
            }
        }
    }

    /// A slanted axis has no stored counterpart: the general reflection is exact to within
    /// rounding, here against the line y = x, which swaps the coordinates.
    #[test]
    fn a_slanted_mirror_is_equal_to_its_literal_reflection_within_rounding() {
        let relation = ConstraintRelation::MirrorLine {
            at: Vec2 { x: 0.0, y: 0.0 },
            angle: 45.0,
        };
        let result = relation
            .apply(Pose2 {
                at: Vec2 {
                    x: -73.127,
                    y: 69.487,
                },
                rotation: 35.0,
            })
            .unwrap();
        assert!((result.at.x - 69.487).abs() < 1e-12);
        assert!((result.at.y + 73.127).abs() < 1e-12);
        assert_eq!(result.rotation, 55.0);
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
