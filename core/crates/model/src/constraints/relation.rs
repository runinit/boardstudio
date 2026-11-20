use crate::model::{Constraint, MirrorAxis, Pose2, Vec2};

pub use crate::model::ConstraintRelation;

/// A directional relation evaluated between the source and target endpoint views.
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
    // `from_legacy` and `inverse` have no production caller yet: the compile-all-records pass and the drag seam land with 114/115.
    pub fn from_legacy(constraint: &Constraint) -> Self {
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
            Constraint::Framed { relation, .. } => relation.clone(),
        }
    }

    pub fn apply(&self, source: Pose2) -> Result<Pose2, String> {
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

    pub fn inverse(&self, target: Pose2) -> Result<Pose2, String> {
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

    pub fn validate(&self) -> Result<(), String> {
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
