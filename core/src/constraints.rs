use crate::model::{Constraint, MirrorAxis, Pose2, ProjectDoc, Vec2};
use std::collections::{BTreeMap, BTreeSet};

pub fn resolve(doc: &mut ProjectDoc) -> Result<Vec<String>, String> {
    let parts: BTreeMap<_, _> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let mut by_target = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for constraint in &doc.constraints {
        if constraint.id().trim().is_empty() || !ids.insert(constraint.id()) {
            return Err(format!(
                "Duplicate or empty constraint ID {}",
                constraint.id()
            ));
        }
        if !parts.contains_key(constraint.source()) || !parts.contains_key(constraint.target()) {
            return Err(format!(
                "Constraint {} references a missing part",
                constraint.id()
            ));
        }
        if by_target
            .insert(constraint.target().to_string(), constraint)
            .is_some()
        {
            return Err(format!(
                "Part {} has multiple constraints",
                constraint.target()
            ));
        }
        let finite = match constraint {
            Constraint::Offset {
                offset, rotation, ..
            } => offset.x.is_finite() && offset.y.is_finite() && rotation.is_finite(),
            Constraint::Mirror { coordinate, .. } => coordinate.is_finite(),
        };
        if !finite {
            return Err(format!(
                "Constraint {} has a non-finite value",
                constraint.id()
            ));
        }
    }

    // Resolve sources before targets, regardless of document order.
    let mut resolved = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    for target in by_target.keys() {
        pose(target, &parts, &by_target, &mut resolved, &mut visiting)?;
    }
    let mut changed = vec![];
    for part in &mut doc.parts {
        if let Some(next) = resolved.get(&part.id)
            && part.pose != *next
        {
            part.pose = *next;
            changed.push(part.id.clone());
        }
    }
    Ok(changed)
}

fn pose(
    id: &str,
    parts: &BTreeMap<String, Pose2>,
    constraints: &BTreeMap<String, &Constraint>,
    resolved: &mut BTreeMap<String, Pose2>,
    visiting: &mut BTreeSet<String>,
) -> Result<Pose2, String> {
    if let Some(value) = resolved.get(id) {
        return Ok(*value);
    }
    if !visiting.insert(id.to_string()) {
        return Err(format!("Constraint cycle includes part {id}"));
    }
    let next = if let Some(constraint) = constraints.get(id) {
        let source = pose(constraint.source(), parts, constraints, resolved, visiting)?;
        match constraint {
            Constraint::Offset {
                offset, rotation, ..
            } => Pose2 {
                at: Vec2 {
                    x: source.at.x + offset.x,
                    y: source.at.y + offset.y,
                },
                rotation: source.rotation + rotation,
            },
            Constraint::Mirror {
                axis, coordinate, ..
            } => Pose2 {
                at: match axis {
                    MirrorAxis::Vertical => Vec2 {
                        x: 2.0 * coordinate - source.at.x,
                        y: source.at.y,
                    },
                    MirrorAxis::Horizontal => Vec2 {
                        x: source.at.x,
                        y: 2.0 * coordinate - source.at.y,
                    },
                },
                // Pose2 records direction only; reflection chirality is not represented.
                rotation: match axis {
                    MirrorAxis::Vertical => 180.0 - source.rotation,
                    MirrorAxis::Horizontal => -source.rotation,
                },
            },
        }
    } else {
        *parts
            .get(id)
            .expect("all constraint references were checked")
    };
    if !next.at.x.is_finite() || !next.at.y.is_finite() || !next.rotation.is_finite() {
        return Err(format!("Constraint target {id} has a non-finite pose"));
    }
    visiting.remove(id);
    resolved.insert(id.to_string(), next);
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Part, Side};

    fn part(id: &str, x: f64, y: f64) -> Part {
        Part {
            id: id.into(),
            definition_id: "key".into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x, y },
                rotation: 15.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn offset(id: &str, source: &str, target: &str) -> Constraint {
        Constraint::Offset {
            id: id.into(),
            source_part_id: source.into(),
            target_part_id: target.into(),
            offset: Vec2 { x: 10.0, y: 2.0 },
            rotation: 5.0,
        }
    }

    #[test]
    fn resolves_chain_and_mirror() {
        let mut doc = ProjectDoc::empty("p", "P");
        doc.parts = vec![
            part("a", 2.0, 3.0),
            part("b", 0.0, 0.0),
            part("c", 0.0, 0.0),
        ];
        doc.constraints = vec![
            Constraint::Mirror {
                id: "m".into(),
                source_part_id: "b".into(),
                target_part_id: "c".into(),
                axis: MirrorAxis::Vertical,
                coordinate: 0.0,
            },
            offset("o", "a", "b"),
        ];
        assert_eq!(resolve(&mut doc).unwrap(), vec!["b", "c"]);
        assert_eq!(doc.parts[1].pose.at, Vec2 { x: 12.0, y: 5.0 });
        assert_eq!(doc.parts[2].pose.at, Vec2 { x: -12.0, y: 5.0 });
        assert_eq!(doc.parts[2].pose.rotation, 160.0);
        doc.parts[0].pose.at.x = 4.0;
        assert_eq!(resolve(&mut doc).unwrap(), vec!["b", "c"]);
        assert_eq!(doc.parts[2].pose.at.x, -14.0);
    }

    #[test]
    fn rejects_invalid_graph_without_mutation() {
        let mut doc = ProjectDoc::empty("p", "P");
        doc.parts = vec![part("a", 1.0, 0.0), part("b", 2.0, 0.0)];
        doc.constraints = vec![offset("a", "a", "b"), offset("b", "b", "a")];
        let before = doc.parts.clone();
        assert!(resolve(&mut doc).unwrap_err().contains("cycle"));
        assert_eq!(doc.parts, before);
        doc.constraints[1] = offset("b", "a", "b");
        assert!(
            resolve(&mut doc)
                .unwrap_err()
                .contains("multiple constraints")
        );
        doc.constraints[1] = offset("b", "missing", "a");
        assert!(resolve(&mut doc).unwrap_err().contains("missing part"));
    }

    #[test]
    fn mirror_angles_follow_reflection_axis() {
        for (axis, source_angle, expected) in [
            (MirrorAxis::Vertical, 0.0, 180.0),
            (MirrorAxis::Vertical, 35.0, 145.0),
            (MirrorAxis::Horizontal, 0.0, 0.0),
            (MirrorAxis::Horizontal, 35.0, -35.0),
        ] {
            let mut doc = ProjectDoc::empty("p", "P");
            let mut source = part("a", 2.0, 3.0);
            source.pose.rotation = source_angle;
            doc.parts = vec![source, part("b", 0.0, 0.0)];
            doc.constraints.push(Constraint::Mirror {
                id: "m".into(),
                source_part_id: "a".into(),
                target_part_id: "b".into(),
                axis,
                coordinate: 0.0,
            });
            resolve(&mut doc).unwrap();
            assert_eq!(doc.parts[1].pose.rotation, expected);
        }
    }
}
