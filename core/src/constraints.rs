use crate::matrix::layout::AssemblyPlacement;
use crate::model::{
    Constraint, EditOperation, MatrixCompanionMode, MirrorAxis, OffsetAxes, Pose2, Position,
    ProjectDoc, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

mod endpoints;
mod relation;

pub fn resolve(doc: &mut ProjectDoc) -> Result<Vec<String>, String> {
    resolve_with_positions(doc, &[])
}

pub(crate) fn resolve_edit(
    doc: &mut ProjectDoc,
    operation: &EditOperation,
) -> Result<Vec<String>, String> {
    let positions = match operation {
        EditOperation::MoveParts { positions } => positions.as_slice(),
        _ => &[],
    };
    resolve_with_positions(doc, positions)
}

fn resolve_with_positions(
    doc: &mut ProjectDoc,
    positions: &[Position],
) -> Result<Vec<String>, String> {
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
            }
            | Constraint::PartOffset {
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

    // Assemblies are implicit local placements; explicit constraints take priority.
    let assemblies = crate::matrix::layout::assembly_placements(doc, positions);
    // Resolve the combined graph source-first instead of relying on repeated passes.
    let mut resolved = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    for target in by_target.keys().chain(assemblies.keys()) {
        pose(
            target,
            &parts,
            &by_target,
            &assemblies,
            &mut resolved,
            &mut visiting,
        )?;
    }
    // Unrelated free parts keep their existing poses without acquiring placement
    // validation semantics from the constraint solver.
    for (id, current) in &parts {
        resolved.entry(id.clone()).or_insert(*current);
    }
    let mut changed = vec![];
    let mut key_moves = BTreeMap::new();
    for part in &mut doc.parts {
        if let Some(next) = resolved.get(&part.id)
            && part.pose != *next
        {
            key_moves.insert(part.id.clone(), (part.pose, *next));
            part.pose = *next;
            changed.push(part.id.clone());
        }
    }
    changed.extend(crate::matrix::layout::reconcile_constraint_keys(
        doc, &key_moves, &resolved,
    )?);
    Ok(changed)
}

fn constraint_pose(constraint: &Constraint, source: Pose2) -> Pose2 {
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
        Constraint::PartOffset {
            offset, rotation, ..
        } => {
            let local = Pose2 {
                at: *offset,
                rotation: *rotation,
            };
            follow_pose(source, local)
        }
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
}

fn follow_pose(source: Pose2, local: Pose2) -> Pose2 {
    let (sin, cos) = source.rotation.to_radians().sin_cos();
    Pose2 {
        at: Vec2 {
            x: source.at.x + local.at.x * cos - local.at.y * sin,
            y: source.at.y + local.at.x * sin + local.at.y * cos,
        },
        rotation: source.rotation + local.rotation,
    }
}

/// Moves a constraint's stored numbers so its target lands `delta` away from where it was
/// and turns `turn` degrees further. An `offset` stores board-axis numbers, so the drag is
/// added as it is. A `part-offset` stores source-axis numbers, so the drag is first turned
/// back into the source's frame. Mirrors reflect rather than offset; callers reject them.
pub(crate) fn retune_offset(constraint: &mut Constraint, source: Pose2, delta: Vec2, turn: f64) {
    match constraint {
        Constraint::Offset {
            offset, rotation, ..
        } => {
            offset.x += delta.x;
            offset.y += delta.y;
            *rotation += turn;
        }
        Constraint::PartOffset {
            offset, rotation, ..
        } => {
            let (sin, cos) = (-source.rotation).to_radians().sin_cos();
            offset.x += delta.x * cos - delta.y * sin;
            offset.y += delta.x * sin + delta.y * cos;
            *rotation += turn;
        }
        Constraint::Mirror { .. } => {
            unreachable!("mirror constraints are rejected before a retune")
        }
    }
}

/// Re-expresses an offset constraint's numbers in `to`'s axes, measured from the source's
/// current pose, so the target does not move. The id and endpoints are kept. Every check
/// runs before the constraint is replaced, so a refusal changes nothing.
pub(crate) fn convert_offset(
    doc: &mut ProjectDoc,
    constraint_id: &str,
    to: OffsetAxes,
) -> Result<Vec<String>, String> {
    let index = doc
        .constraints
        .iter()
        .position(|constraint| constraint.id() == constraint_id)
        .ok_or_else(|| format!("Constraint {constraint_id} does not exist"))?;
    let source_id = doc.constraints[index].source().to_string();
    let source = doc
        .parts
        .iter()
        .find(|part| part.id == source_id)
        .ok_or_else(|| format!("Constraint {constraint_id} references a missing part"))?
        .pose;
    let (sin, cos) = source.rotation.to_radians().sin_cos();
    let turn = |v: Vec2, sin: f64, cos: f64| Vec2 {
        x: v.x * cos - v.y * sin,
        y: v.x * sin + v.y * cos,
    };
    let next = match (doc.constraints[index].clone(), to) {
        (
            Constraint::Offset {
                id,
                source_part_id,
                target_part_id,
                offset,
                rotation,
            },
            OffsetAxes::Part,
        ) => Constraint::PartOffset {
            id,
            source_part_id,
            target_part_id,
            offset: turn(offset, -sin, cos),
            rotation,
        },
        (
            Constraint::PartOffset {
                id,
                source_part_id,
                target_part_id,
                offset,
                rotation,
            },
            OffsetAxes::Board,
        ) => Constraint::Offset {
            id,
            source_part_id,
            target_part_id,
            offset: turn(offset, sin, cos),
            rotation,
        },
        (current @ (Constraint::Offset { .. } | Constraint::PartOffset { .. }), _) => current,
        (Constraint::Mirror { .. }, _) => {
            return Err(format!(
                "Constraint {constraint_id} is a mirror and has no offset axes"
            ));
        }
    };
    doc.constraints[index] = next;
    Ok(vec![constraint_id.into()])
}

fn pose(
    id: &str,
    parts: &BTreeMap<String, Pose2>,
    constraints: &BTreeMap<String, &Constraint>,
    assemblies: &BTreeMap<String, AssemblyPlacement>,
    resolved: &mut BTreeMap<String, Pose2>,
    visiting: &mut BTreeSet<String>,
) -> Result<Pose2, String> {
    if let Some(value) = resolved.get(id) {
        return Ok(*value);
    }
    if !visiting.insert(id.to_string()) {
        return Err(format!("Constraint cycle includes part {id}"));
    }
    let next = if let Some(assembly) = assemblies.get(id) {
        // The classification already resolved explicit-constraint precedence, so
        // a constrained companion settles from its constraint and the others
        // follow the key (an authored pose only while the key is unchanged).
        match &assembly.mode {
            MatrixCompanionMode::Constrained { .. } => {
                let constraint = constraints
                    .get(id)
                    .ok_or_else(|| format!("Constraint for companion {id} is missing"))?;
                let source = pose(
                    constraint.source(),
                    parts,
                    constraints,
                    assemblies,
                    resolved,
                    visiting,
                )?;
                constraint_pose(constraint, source)
            }
            MatrixCompanionMode::Authored | MatrixCompanionMode::Follows => {
                let source = pose(
                    &assembly.parent_id,
                    parts,
                    constraints,
                    assemblies,
                    resolved,
                    visiting,
                )?;
                if matches!(assembly.mode, MatrixCompanionMode::Authored)
                    && let Some(authored) = assembly.authored_pose
                    && parts.get(&assembly.parent_id) == Some(&source)
                {
                    authored
                } else {
                    follow_pose(source, assembly.local_pose)
                }
            }
        }
    } else if let Some(constraint) = constraints.get(id) {
        let source = pose(
            constraint.source(),
            parts,
            constraints,
            assemblies,
            resolved,
            visiting,
        )?;
        constraint_pose(constraint, source)
    } else {
        *parts
            .get(id)
            .expect("all placement references were checked")
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
