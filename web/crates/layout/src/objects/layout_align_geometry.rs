//! Pure accepted-envelope and axis-alignment policy for Layout Align.
use boardstudio_core::model::{Matrix, Mirror, Part, PartDefinition, PartKind, Vec2};

pub fn reference_choice(
    current: Option<&str>,
    eligible: &[String],
    authoritative: bool,
) -> Option<String> {
    if !authoritative {
        return current.map(str::to_owned);
    }
    current
        .filter(|id| eligible.iter().any(|eligible| eligible == id))
        .map(str::to_owned)
        .or_else(|| eligible.first().cloned())
}

pub fn reconcile_reference_choice(
    current: Option<&str>,
    eligible: &[String],
    authoritative: bool,
    mut write: impl FnMut(Option<String>),
) -> Option<String> {
    let next = reference_choice(current, eligible, authoritative);
    if authoritative && next.as_deref() != current {
        write(next.clone());
    }
    next
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignCommand {
    Left,
    CenterX,
    Right,
    Top,
    CenterY,
    Bottom,
}

impl AlignCommand {
    pub const ALL: [Self; 6] = [
        Self::Left,
        Self::CenterX,
        Self::Right,
        Self::Top,
        Self::CenterY,
        Self::Bottom,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::CenterX => "Center X",
            Self::Right => "Right",
            Self::Top => "Top",
            Self::CenterY => "Center Y",
            Self::Bottom => "Bottom",
        }
    }

    fn axis_anchor(self) -> (Axis, Anchor) {
        match self {
            Self::Left => (Axis::X, Anchor::Min),
            Self::CenterX => (Axis::X, Anchor::Center),
            Self::Right => (Axis::X, Anchor::Max),
            Self::Top => (Axis::Y, Anchor::Max),
            Self::CenterY => (Axis::Y, Anchor::Center),
            Self::Bottom => (Axis::Y, Anchor::Min),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Anchor {
    Min,
    Center,
    Max,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl Bounds {
    fn from_points(points: &[Vec2]) -> Option<Self> {
        let first = *points.first()?;
        if !finite_point(first) {
            return None;
        }
        let mut bounds = Self {
            min_x: first.x,
            max_x: first.x,
            min_y: first.y,
            max_y: first.y,
        };
        for point in &points[1..] {
            if !finite_point(*point) {
                return None;
            }
            bounds.min_x = bounds.min_x.min(point.x);
            bounds.max_x = bounds.max_x.max(point.x);
            bounds.min_y = bounds.min_y.min(point.y);
            bounds.max_y = bounds.max_y.max(point.y);
        }
        Some(bounds)
    }

    fn anchor(self, axis: Axis, anchor: Anchor) -> f64 {
        let (min, max) = match axis {
            Axis::X => (self.min_x, self.max_x),
            Axis::Y => (self.min_y, self.max_y),
        };
        match anchor {
            Anchor::Min => min,
            Anchor::Center => (min + max) / 2.0,
            Anchor::Max => max,
        }
    }
}

fn finite_point(point: Vec2) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

#[cfg(test)]
mod state_tests {
    use super::{reconcile_reference_choice, reference_choice};

    #[test]
    fn reference_choice_retains_preferences_when_projection_is_unavailable() {
        assert_eq!(
            reference_choice(Some("B"), &["A".into()], false),
            Some("B".into())
        );
        assert_eq!(
            reference_choice(Some("B"), &["A".into(), "B".into()], true),
            Some("B".into())
        );
        assert_eq!(
            reference_choice(Some("B"), &["A".into()], true),
            Some("A".into())
        );
        assert_eq!(reference_choice(None, &[], true), None);
    }

    #[test]
    fn reconciliation_writes_only_changed_authoritative_choices() {
        let mut writes = Vec::new();
        assert_eq!(
            reconcile_reference_choice(None, &[], true, |next| writes.push(next)),
            None
        );
        assert_eq!(writes, Vec::<Option<String>>::new());
        assert_eq!(
            reconcile_reference_choice(Some("B"), &["A".into()], false, |next| {
                writes.push(next)
            }),
            Some("B".into())
        );
        assert!(writes.is_empty());
        assert_eq!(
            reconcile_reference_choice(Some("B"), &["A".into()], true, |next| {
                writes.push(next)
            }),
            Some("A".into())
        );
        assert_eq!(writes, vec![Some("A".into())]);
    }
}

/// Resolve the exact accepted envelope used by React selectionOutline for one live part.
pub fn transformed_envelope(part: &Part, definition: &PartDefinition) -> Result<Vec<Vec2>, String> {
    let local = if matches!(&definition.kind, PartKind::Switch) {
        match part.keycap.or(definition.keycap) {
            Some(size) if finite_point(size) && size.x > 0.0 && size.y > 0.0 => {
                vec![
                    Vec2 {
                        x: -size.x / 2.0,
                        y: -size.y / 2.0,
                    },
                    Vec2 {
                        x: size.x / 2.0,
                        y: -size.y / 2.0,
                    },
                    Vec2 {
                        x: size.x / 2.0,
                        y: size.y / 2.0,
                    },
                    Vec2 {
                        x: -size.x / 2.0,
                        y: size.y / 2.0,
                    },
                ]
            }
            Some(_) => {
                return Err(format!(
                    "{} has no supported alignment envelope",
                    part.reference
                ));
            }
            None => definition.courtyard.clone(),
        }
    } else {
        definition.courtyard.clone()
    };
    if local.len() < 3 || local.iter().any(|point| !finite_point(*point)) {
        return Err(format!(
            "{} has no supported alignment envelope",
            part.reference
        ));
    }
    if !finite_point(part.pose.at) || !part.pose.rotation.is_finite() {
        return Err(format!("{} has an invalid accepted pose", part.reference));
    }
    let angle = part.pose.rotation.to_radians();
    let (sin, cos) = angle.sin_cos();
    Ok(local
        .into_iter()
        .map(|point| Vec2 {
            x: part.pose.at.x + point.x * cos - point.y * sin,
            y: part.pose.at.y + point.x * sin + point.y * cos,
        })
        .collect())
}

pub fn alignment_delta(
    moving: &[Vec<Vec2>],
    reference: &[Vec2],
    command: AlignCommand,
) -> Result<Vec2, String> {
    let mut points = Vec::new();
    for envelope in moving {
        points.extend_from_slice(envelope);
    }
    let moving_bounds = Bounds::from_points(&points)
        .ok_or_else(|| "The selected parts have no supported alignment envelope".to_owned())?;
    let reference_bounds = Bounds::from_points(reference)
        .ok_or_else(|| "The chosen reference has no supported alignment envelope".to_owned())?;
    let (axis, anchor) = command.axis_anchor();
    let delta = reference_bounds.anchor(axis, anchor) - moving_bounds.anchor(axis, anchor);
    if !delta.is_finite() {
        return Err("The alignment distance is not finite".into());
    }
    Ok(match axis {
        Axis::X => Vec2 { x: delta, y: 0.0 },
        Axis::Y => Vec2 { x: 0.0, y: delta },
    })
}

pub fn local_matrix_delta(matrix: &Matrix, delta: Vec2, column: usize) -> Vec2 {
    let angle = -matrix.rotation.unwrap_or_default().to_radians();
    let (sin, cos) = angle.sin_cos();
    let mut x = delta.x * cos - delta.y * sin;
    let mut y = delta.x * sin + delta.y * cos;
    match matrix.mirror {
        Some(Mirror::None) | None => {}
        Some(Mirror::X) => x *= -1.0,
        Some(Mirror::Y) => y *= -1.0,
    }
    let splay = -matrix
        .column_splays
        .iter()
        .take(column + 1)
        .sum::<f64>()
        .to_radians();
    let (sin, cos) = splay.sin_cos();
    Vec2 {
        x: x * cos - y * sin,
        y: x * sin + y * cos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::Pose2;

    fn part(definition_id: &str, keycap: Option<Vec2>, rotation: f64) -> Part {
        Part {
            keycap,
            outline: None,
            id: "part-1".into(),
            definition_id: definition_id.into(),
            reference: "S1".into(),
            pose: Pose2 {
                at: Vec2 { x: 10.0, y: 20.0 },
                rotation,
            },
            side: boardstudio_core::model::Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn definition(kind: &str, keycap: Option<Vec2>, courtyard: Vec<Vec2>) -> PartDefinition {
        let mut value = serde_json::json!({
            "id": "definition-1",
            "name": "Definition",
            "kind": kind,
            "courtyard": courtyard,
            "pads": []
        });
        if let Some(keycap) = keycap {
            value["keycap"] = serde_json::json!(keycap);
        }
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn six_anchors_align_the_combined_axis_aligned_bounds() {
        assert_eq!(AlignCommand::ALL.len(), 6);
        assert_eq!(
            AlignCommand::ALL.map(AlignCommand::label),
            ["Left", "Center X", "Right", "Top", "Center Y", "Bottom"]
        );
        let moving = vec![
            vec![Vec2 { x: 0.0, y: -2.0 }, Vec2 { x: 4.0, y: 2.0 }],
            vec![Vec2 { x: 5.0, y: -1.0 }, Vec2 { x: 9.0, y: 3.0 }],
        ];
        let reference = vec![Vec2 { x: 20.0, y: 10.0 }, Vec2 { x: 28.0, y: 18.0 }];
        assert_eq!(
            alignment_delta(&moving, &reference, AlignCommand::Left).unwrap(),
            Vec2 { x: 20.0, y: 0.0 }
        );
        assert_eq!(
            alignment_delta(&moving, &reference, AlignCommand::CenterX).unwrap(),
            Vec2 { x: 19.5, y: 0.0 }
        );
        assert_eq!(
            alignment_delta(&moving, &reference, AlignCommand::Right).unwrap(),
            Vec2 { x: 19.0, y: 0.0 }
        );
        assert_eq!(
            alignment_delta(&moving, &reference, AlignCommand::Top).unwrap(),
            Vec2 { x: 0.0, y: 15.0 }
        );
        assert_eq!(
            alignment_delta(&moving, &reference, AlignCommand::CenterY).unwrap(),
            Vec2 { x: 0.0, y: 13.5 }
        );
        assert_eq!(
            alignment_delta(&moving, &reference, AlignCommand::Bottom).unwrap(),
            Vec2 { x: 0.0, y: 12.0 }
        );
    }

    #[test]
    fn column_alignment_delta_uses_matrix_rotation_mirror_and_accumulated_splay() {
        let matrix: Matrix = serde_json::from_value(serde_json::json!({
            "id":"m", "rows":2, "columns":3, "pitch":{"x":19,"y":19},
            "origin":{"x":0,"y":0}, "definitionId":"d", "partIds":[],
            "mirror":"x", "rotation":90, "columnSplays":[0,10]
        }))
        .unwrap();
        let actual = local_matrix_delta(&matrix, Vec2 { x: 2.0, y: 3.0 }, 1);
        assert!((actual.x - -3.3017196143704846).abs() < 1e-12);
        assert!((actual.y - -1.4486709730236247).abs() < 1e-12);
    }

    #[test]
    fn transformed_switch_keycap_override_uses_four_rotated_corners() {
        let keycap = Vec2 { x: 8.0, y: 4.0 };
        let part = part("definition-1", Some(keycap), 90.0);
        let definition = definition("switch", Some(Vec2 { x: 19.0, y: 19.0 }), vec![]);
        let corners = transformed_envelope(&part, &definition).unwrap();
        assert_eq!(corners.len(), 4);
        let bounds = Bounds::from_points(&corners).unwrap();
        assert!((bounds.min_x - 8.0).abs() < 1e-12);
        assert!((bounds.max_x - 12.0).abs() < 1e-12);
        assert!((bounds.min_y - 16.0).abs() < 1e-12);
        assert!((bounds.max_y - 24.0).abs() < 1e-12);
    }

    #[test]
    fn missing_exact_envelope_and_nonfinite_bounds_are_rejected() {
        let part = part("definition-1", None, 0.0);
        let definition = definition("switch", None, vec![]);
        assert!(transformed_envelope(&part, &definition).is_err());
        assert!(alignment_delta(&[], &[Vec2 { x: 1.0, y: 1.0 }], AlignCommand::Left).is_err());
        assert!(
            Bounds::from_points(&[Vec2 {
                x: f64::NAN,
                y: 0.0
            }])
            .is_none()
        );
    }
}
