use crate::matrix::layout::AssemblyPlacement;
use crate::matrix::splay::rotate;
use crate::model::{
    Constraint, EditOperation, MatrixCompanionMode, MirrorAxis, OffsetAxes, Part, Pose2, Position,
    ProjectDoc, Vec2,
};
use endpoints::{EndpointFrame, FramedConstraint};
use relation::ConstraintRelation;
use std::collections::{BTreeMap, BTreeSet};

mod endpoints;
#[cfg(test)]
mod framed_tests;
mod relation;
mod retune;

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
    resolve_prepared(doc, positions, &[])
}

/// A record the solver evaluates for one authoritative target part.
enum Driver<'a> {
    /// A stored Offset, PartOffset or Mirror, evaluated with its exact legacy arithmetic.
    Legacy(&'a Constraint),
    /// A framed relation bound to the live document.
    Framed(Framed<'a>),
}

struct Framed<'a> {
    id: &'a str,
    source_authority: String,
    source_frame: EndpointFrame,
    target_frame: EndpointFrame,
    source_owner: Option<String>,
    target_owner: Option<String>,
    relation: &'a ConstraintRelation,
}

impl Driver<'_> {
    fn source_authority(&self) -> &str {
        match self {
            Self::Legacy(constraint) => constraint.source(),
            Self::Framed(framed) => &framed.source_authority,
        }
    }

    /// The target authority's pose given the resolved source authority's pose. `current` is the
    /// target's stored pose, which chooses between headings a full turn apart.
    fn evaluate(
        &self,
        source: Pose2,
        owners: [Option<Pose2>; 2],
        current: Pose2,
    ) -> Result<Pose2, String> {
        match self {
            Self::Legacy(constraint) => Ok(constraint_pose(constraint, source)),
            Self::Framed(framed) => endpoints::solve_target(
                source,
                framed.source_frame.with_key(owners[0]),
                framed.target_frame.with_key(owners[1]),
                framed.relation,
                current,
            )
            .map_err(|message| format!("Constraint {}: {message}", framed.id)),
        }
    }
}

/// Resolves the document's stored constraints together with `framed` relations in one
/// source-first graph. Framed relations are private to this module until the persisted arm is
/// activated; a document never supplies them, so production resolution is unchanged.
fn resolve_prepared(
    doc: &mut ProjectDoc,
    positions: &[Position],
    framed: &[FramedConstraint],
) -> Result<Vec<String>, String> {
    let parts: BTreeMap<_, _> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let mut by_target: BTreeMap<String, Driver> = BTreeMap::new();
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
            .insert(constraint.target().to_string(), Driver::Legacy(constraint))
            .is_some()
        {
            return Err(format!(
                "Part {} has multiple constraints",
                constraint.target()
            ));
        }
        if !constraint.is_finite() {
            return Err(format!(
                "Constraint {} has a non-finite value",
                constraint.id()
            ));
        }
    }
    for record in framed {
        let FramedConstraint::Framed {
            id,
            source,
            target,
            relation,
        } = record;
        if id.trim().is_empty() || !ids.insert(id.as_str()) {
            return Err(format!("Duplicate or empty constraint ID {id}"));
        }
        let source = endpoints::resolve_endpoint(doc, id, "source", source)?;
        let target = endpoints::resolve_endpoint(doc, id, "target", target)?;
        relation
            .validate()
            .map_err(|_| format!("Constraint {id} has a non-finite value"))?;
        // Authorities are what the graph orders: two visible targets of one authority, or a
        // visible source that is its own target's authority, are one part.
        let driver = Driver::Framed(Framed {
            id,
            source_authority: source.authoritative_id,
            source_frame: source.frame,
            target_frame: target.frame,
            source_owner: source.owner,
            target_owner: target.owner,
            relation,
        });
        if by_target
            .insert(target.authoritative_id.clone(), driver)
            .is_some()
        {
            return Err(format!(
                "Part {} has multiple constraints",
                target.authoritative_id
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

/// The pose of the part a constraint is measured from.
pub(crate) fn source_pose(parts: &[Part], constraint: &Constraint) -> Result<Pose2, String> {
    parts
        .iter()
        .find(|part| part.id == constraint.source())
        .map(|part| part.pose)
        .ok_or_else(|| format!("Constraint {} source is missing", constraint.id()))
}

/// What `retune_offset` did.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Retune {
    Applied,
    /// The constraint is a mirror, which reflects rather than offsets. Nothing changed.
    Mirror,
}

/// Why a part driven by a mirror constraint cannot be dragged.
pub(crate) fn mirror_drag_refusal(part_id: &str) -> String {
    format!("Part {part_id} is controlled by a mirror constraint and cannot be dragged")
}

/// Moves a constraint's stored numbers so its target lands `delta` away from where it was
/// and, when `turn` is given, turns that many degrees further. `None` leaves the stored
/// rotation untouched, bit for bit: adding `0.0` would rewrite a stored `-0.0`. A drag
/// passes `None`; `dragging_a_held_companion_keeps_the_exact_stored_rotation_of_either_kind`
/// in `core/tests/part_offset_constraint.rs` pins it. An `offset` stores board-axis numbers,
/// so the drag is added as it is. A `part-offset` stores source-axis numbers, so the drag is
/// first turned back into the source's frame, measured from the source's pose in `parts`.
pub(crate) fn retune_offset(
    parts: &[Part],
    constraint: &mut Constraint,
    delta: Vec2,
    turn: Option<f64>,
) -> Result<Retune, String> {
    if constraint.is_mirror() {
        return Ok(Retune::Mirror);
    }
    let source = source_pose(parts, constraint)?;
    let (offset, rotation, step) = match constraint {
        Constraint::Offset {
            offset, rotation, ..
        } => (offset, rotation, delta),
        Constraint::PartOffset {
            offset, rotation, ..
        } => (
            offset,
            rotation,
            rotate(delta, -source.rotation, Vec2::default()),
        ),
        Constraint::Mirror { .. } => return Ok(Retune::Mirror),
    };
    offset.x += step.x;
    offset.y += step.y;
    if let Some(turn) = turn {
        *rotation += turn;
    }
    Ok(Retune::Applied)
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
    let source = source_pose(&doc.parts, &doc.constraints[index])?;
    let target = doc.constraints[index].target().to_string();
    if !doc.parts.iter().any(|part| part.id == target) {
        return Err(format!("Constraint {constraint_id} target is missing"));
    }
    let current = &doc.constraints[index];
    let (Some(from), Some((offset, rotation))) = (current.offset_axes(), current.offset_numbers())
    else {
        return Err(format!(
            "Constraint {constraint_id} is a mirror and has no offset axes"
        ));
    };
    if from != to {
        let (id, source_part_id, target_part_id) = (
            current.id().to_owned(),
            current.source().to_owned(),
            current.target().to_owned(),
        );
        let into_axes = match to {
            OffsetAxes::Part => -source.rotation,
            OffsetAxes::Board => source.rotation,
        };
        doc.constraints[index] = Constraint::offset(
            to,
            id,
            source_part_id,
            target_part_id,
            rotate(offset, into_axes, Vec2::default()),
            rotation,
        );
    }
    Ok(vec![constraint_id.into()])
}

/// Evaluates the record that drives `id`: its source first, then any key a companion endpoint
/// is projected against, so the projection reads live poses and cycles through it are refused.
fn drive(
    id: &str,
    driver: &Driver,
    parts: &BTreeMap<String, Pose2>,
    constraints: &BTreeMap<String, Driver>,
    assemblies: &BTreeMap<String, AssemblyPlacement>,
    resolved: &mut BTreeMap<String, Pose2>,
    visiting: &mut BTreeSet<String>,
) -> Result<Pose2, String> {
    let current = *parts
        .get(id)
        .ok_or_else(|| format!("Constraint target {id} is missing"))?;
    let source = pose(
        driver.source_authority(),
        parts,
        constraints,
        assemblies,
        resolved,
        visiting,
    )?;
    let mut owners = [None, None];
    if let Driver::Framed(framed) = driver {
        for (slot, owner) in [&framed.source_owner, &framed.target_owner]
            .into_iter()
            .enumerate()
        {
            if let Some(owner) = owner {
                owners[slot] = Some(pose(
                    owner,
                    parts,
                    constraints,
                    assemblies,
                    resolved,
                    visiting,
                )?);
            }
        }
    }
    driver.evaluate(source, owners, current)
}

fn pose(
    id: &str,
    parts: &BTreeMap<String, Pose2>,
    constraints: &BTreeMap<String, Driver>,
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
    // An explicit framed relation outranks the implicit companion placement, like a stored one.
    let framed = matches!(constraints.get(id), Some(Driver::Framed(_)));
    let next = if let Some(assembly) = assemblies.get(id).filter(|_| !framed) {
        // The classification already resolved explicit-constraint precedence, so
        // a constrained companion settles from its constraint and the others
        // follow the key (an authored pose only while the key is unchanged).
        match &assembly.mode {
            MatrixCompanionMode::Constrained { .. } => {
                let driver = constraints
                    .get(id)
                    .ok_or_else(|| format!("Constraint for companion {id} is missing"))?;
                drive(
                    id,
                    driver,
                    parts,
                    constraints,
                    assemblies,
                    resolved,
                    visiting,
                )?
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
    } else if let Some(driver) = constraints.get(id) {
        drive(
            id,
            driver,
            parts,
            constraints,
            assemblies,
            resolved,
            visiting,
        )?
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

    #[test]
    fn resolve_refuses_a_non_finite_number_of_any_kind_without_mutation() {
        let kinds = |x: f64, y: f64, turn: f64| {
            let offset = Vec2 { x, y };
            [
                Constraint::Offset {
                    id: "o".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    offset,
                    rotation: turn,
                },
                Constraint::PartOffset {
                    id: "o".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    offset,
                    rotation: turn,
                },
                Constraint::Mirror {
                    id: "o".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    axis: MirrorAxis::Vertical,
                    coordinate: x + y + turn,
                },
            ]
        };
        for bad in [f64::NAN, f64::INFINITY] {
            for (x, y, turn) in [(bad, 0.0, 0.0), (0.0, bad, 0.0), (0.0, 0.0, bad)] {
                for constraint in kinds(x, y, turn) {
                    let mut doc = ProjectDoc::empty("p", "P");
                    doc.parts = vec![part("a", 1.0, 0.0), part("b", 2.0, 0.0)];
                    doc.constraints = vec![constraint.clone()];
                    let before = doc.parts.clone();
                    let message = resolve(&mut doc).unwrap_err();
                    assert!(
                        message.contains("non-finite"),
                        "{constraint:?} gave {message}"
                    );
                    // The constraint holds a NaN, so compare the parts it would have moved.
                    assert_eq!(doc.parts, before);
                }
            }
        }
    }

    /// An accepted document never holds a constraint with a missing endpoint, so the edit API
    /// cannot reach these guards; a conversion must still refuse and change nothing if one is
    /// ever handed such a document.
    #[test]
    fn convert_offset_refuses_a_missing_endpoint_and_changes_nothing() {
        for (missing, source, target) in [("source", "ghost", "b"), ("target", "a", "ghost")] {
            for to in [OffsetAxes::Board, OffsetAxes::Part] {
                let mut doc = ProjectDoc::empty("p", "P");
                doc.parts = vec![part("a", 2.0, 3.0), part("b", 12.0, 5.0)];
                doc.constraints = vec![offset("o", source, target)];
                let before = doc.clone();
                let message = convert_offset(&mut doc, "o", to).unwrap_err();
                assert!(
                    message.contains(missing) && message.contains("missing"),
                    "{message}"
                );
                assert_eq!(doc, before, "a missing {missing} changed the document");
            }
        }
    }
}
